---
name: wasp-debug
description: Debug a UOMI WASM agent. Diagnose and fix errors across the full stack — Rust compilation, WASM runtime panics, LLM API failures, host configuration, and on-chain execution errors.
argument-hint: [error message or symptom]
when_to_use: Trigger when the developer reports an error, unexpected output, build failure, panic, API error, empty output, or on-chain execution failure in a UOMI agent.
allowed-tools: Read Edit Bash(cat*) Bash(ls*) Bash(cargo build*) Bash(npm run build*) Bash(npm start*) Bash(rustc --version) Bash(rustup target list*)
---

# UOMI Agent Debugger

You are debugging a UOMI WASM agent. The developer's error or symptom:

**$ARGUMENTS**

---

## Step 0 — Gather context

Before diagnosing, read the relevant files if you haven't already:

```
agent-template/src/lib.rs       ← agent logic
agent-template/cargo.toml       ← Rust dependencies and package name
uomi.config.json                ← model configuration
host/src/output.txt             ← last raw output written by the agent (if exists)
```

Also ask the developer to paste the **full terminal output** of the failing command if they haven't already — error messages from the host are critical.

---

## Error catalog — match the symptom and jump to the section

| Symptom | Section |
|---------|---------|
| `error[E0463]: can't find crate for 'std'` | A1 |
| `error: no matching package named 'X'` / crate not found | A2 |
| WASM file doesn't compile / `cargo build` fails | A3 |
| `File ./agent_template.wasm does not exist` | B1 |
| `File ./input.txt does not exist` | B2 |
| `Error executing WASM: ...` / `Trap: unreachable` | C1 |
| `Failed to get memory export` / `Failed to write memory` | C2 |
| `get_typed_func ... "run"` error / missing export | C3 |
| `thread 'main' panicked` in WASM | C4 |
| `Invalid model ID` / `Model not found` | D1 |
| `Connection refused` / `Network error` / `error sending request` | D2 |
| `Request failed: 401` | D3 |
| `Request failed: 404` or wrong response format | D4 |
| `JSON error` / `serde_json` parse failure | D5 |
| `Request failed (attempt 3/3)` — retries exhausted | D6 |
| Agent runs but output is empty | E1 |
| Agent runs but output is garbled / wrong format | E2 |
| `call_ai_service` returns unexpected content | E3 |
| IPFS fetch returns empty or hangs | E4 |
| On-chain `getAgentOutput` returns `success: false` | F1 |
| On-chain `callAgent` transaction reverts | F2 |
| On-chain request never completes (`completed: false`) | F3 |

---

## A — Compilation errors

### A1 — `error[E0463]: can't find crate for 'std'` or wasm32 target missing

The WASM compilation target is not installed.

**Fix:**
```bash
rustup target add wasm32-unknown-unknown
rustup target list --installed | grep wasm32
```

Then rebuild:
```bash
npm run build
```

---

### A2 — Crate not found / dependency error

A crate in `cargo.toml` either doesn't exist, has the wrong version, or is incompatible with `wasm32-unknown-unknown`.

**Check:** does the crate work in WASM? Many crates that use OS primitives (threads, filesystem, network sockets) won't compile to WASM.

**Crates that work:** `serde`, `serde_json`, `base64`, `hex`, `sha2`, `sha3`, `regex`, `chrono` (with `wasmbind` feature).

**Crates that won't work:** `reqwest`, `tokio`, `hyper`, `std::fs`, `rand` (without WASM feature).

**Fix:** remove the incompatible crate from `cargo.toml` and use the UOMI host functions instead (`call_ai_service`, `get_cid_file_service`).

---

### A3 — General `cargo build` failure

Read the full error. Rust compiler errors always include a `-->` with file:line pointing to the exact problem.

Common causes:
- **Type mismatch**: check that `Message` has `#[derive(Serialize, Deserialize)]`
- **Missing `mod utils;`** at the top of `lib.rs`
- **Wrong `crate-type`**: `cargo.toml` must have `crate-type = ["cdylib"]`
- **Package name mismatch**: name in `cargo.toml` must use `_` not `-` for WASM output filename

Check the package name:
```bash
cat agent-template/cargo.toml | grep "^name"
```
If name is `my-agent`, the WASM output is `my_agent.wasm`. The build script copies `$DIR.wasm` where `DIR` is derived from the directory name — they must match.

---

## B — Host setup errors

### B1 — `File ./agent_template.wasm does not exist`

The WASM hasn't been compiled yet, or the build script didn't copy it correctly.

**Fix:**
```bash
npm run build
ls -lh host/src/agent_template.wasm
```

If the file is missing after build, check the build script:
```bash
cat bin/build_and_run_host.sh
```
The script derives the filename from the directory name:
```bash
DIR=${PWD##*/}
DIR=${DIR//-/_}
cp ./target/wasm32-unknown-unknown/release/$DIR.wasm ./host/src/agent_template.wasm
```
The `cargo.toml` package name must produce the same `$DIR.wasm` filename.

---

### B2 — `File ./input.txt does not exist`

The host reads `host/src/input.txt` as the agent's input. It's not created automatically.

**Fix:** `npm start` (main.js) creates `input.txt` from the conversation history. If running the host directly, create it manually:
```bash
echo '[{"role":"user","content":"Hello"}]' > host/src/input.txt
```

---

## C — WASM runtime errors

### C1 — `Error executing WASM` / `Trap: unreachable` / wasmi error

The WASM module crashed during execution. This is almost always a **Rust panic** inside `run()`.

**Diagnosis:** add `utils::log()` calls throughout `lib.rs` to narrow down where it panics:
```rust
#[no_mangle]
pub extern "C" fn run() {
    utils::log("run() started");
    let input = utils::read_input();
    utils::log(&format!("input len: {}", input.len()));
    let messages = utils::parse_messages(&input);  // ← likely panic here if input malformed
    utils::log("messages parsed");
    // ...
}
```

Look for `[WASM]` log lines in the terminal — the last one before the crash points to the problem.

**Common panic locations:**
- `utils::parse_messages(&input)` — panics if input is not a valid JSON messages array → use safe parsing (see `fixes/safe_patterns.rs`)
- `.unwrap()` on any `Result` or `Option` — replace with `.unwrap_or_else(|e| { utils::log(...); ... })`
- `data[4..data_len + 4]` in `extract_wasm_data` — means a host function returned malformed data (usually an API call failed and the panic propagates)

---

### C2 — `Failed to get memory export` / `Failed to write memory`

This is a panic inside the **host** (not your Rust code), triggered when a host function can't access WASM memory. Almost always caused by a WASM module that didn't export memory properly.

**Fix:** make sure `cargo.toml` uses `crate-type = ["cdylib"]` and not `["bin"]` or `["lib"]`.

---

### C3 — `get_typed_func ... "run"` failed

The host can't find the `run` function export in the WASM.

**Fix:** the entry point must be exactly:
```rust
#[no_mangle]
pub extern "C" fn run() {
```

Both `#[no_mangle]` and `pub extern "C"` are required. Check that `lib.rs` has this exact signature.

---

### C4 — `thread 'main' panicked` (from host, not WASM)

This happens when a host function itself panics — most commonly `call_service_api(...).unwrap()` when the API call fails.

The underlying API error is printed before the panic. Look for lines like:
```
Request failed (attempt 1/3): 401 Unauthorized
```

Then follow section **D** to fix the API issue.

---

## D — LLM API errors

### D1 — `Invalid model ID` / model not found in config

The model number passed to `call_ai_service(N, ...)` doesn't exist in `uomi.config.json`.

**Check:**
```bash
cat uomi.config.json
```
Model keys are strings (`"1"`, `"2"`, ...). Make sure the number in your Rust code matches.

---

### D2 — `Connection refused` / `Network error` / `error sending request to http://localhost:8888`

The host is trying to reach the UOMI node-ai service at `http://localhost:8888` (default for model 1 without a custom URL) but nothing is running there.

**For local dev — use an external LLM instead:**

Edit `uomi.config.json` to add a model with an explicit URL:
```json
"2": {
  "name": "gpt-4o-mini",
  "url": "https://api.openai.com/v1/chat/completions",
  "api_key": "sk-..."
}
```

Then in `lib.rs` call model `2`:
```rust
let response = utils::call_ai_service(2, utils::prepare_request(&body));
```

**For production — run node-ai:**
See https://github.com/Uomi-network/uomi-node-ai

---

### D3 — `Request failed: 401 Unauthorized`

Wrong or missing API key for an external model.

**Fix:** check `api_key` in `uomi.config.json` for the model being called. Make sure there are no extra spaces or newlines.

```json
"2": {
  "name": "gpt-4o-mini",
  "url": "https://api.openai.com/v1/chat/completions",
  "api_key": "sk-proj-..."  ← must be the full key, no quotes around it in JSON
}
```

---

### D4 — `Request failed: 404` or unexpected response format

The `url` in `uomi.config.json` is wrong, or the model name doesn't match what the API expects.

**OpenAI-compatible endpoint format:**
```
POST https://api.openai.com/v1/chat/completions
body: { "model": "<name>", "messages": [...] }
```

The host uses the `url` field directly. Common mistakes:
- Missing `/v1/chat/completions` path
- Using a completion endpoint instead of chat (`/v1/completions` vs `/v1/chat/completions`)
- Wrong base URL for third-party providers (Groq, Together, Anthropic use different paths)

---

### D5 — `JSON error` / `serde_json parse failure`

The request body sent to the LLM is malformed JSON.

**Check your `body` string** before calling `call_ai_service`:
```rust
let body = format!("{{\"messages\": {}}}", serde_json::to_string(&messages).unwrap());
utils::log(&format!("Request body: {}", body));  // ← add this temporarily
```

The body **must** have a `messages` key wrapping the array:
```json
{"messages": [{"role": "system", "content": "..."}, {"role": "user", "content": "..."}]}
```

---

### D6 — `Request failed (attempt 3/3)` — all retries exhausted

The API is reachable but consistently failing. Check the status code:
- `429 Too Many Requests` → rate limit hit, add a delay or use a different API key
- `500 Internal Server Error` → server-side issue, try a different model/provider
- `503 Service Unavailable` → provider overloaded

You can increase retries in `uomi.config.json`:
```json
"api": { "timeout_ms": 60000, "retry_attempts": 5 }
```

---

## E — Logic errors

### E1 — Agent runs but output is empty

`save_output` was never called, or was called with an empty slice.

**Verify:**
```bash
cat host/src/output.txt
```

**Fix:** make sure every code path calls `save_output` exactly once:
```rust
// Always at the end, even on error
utils::save_output(&response_bytes);

// Or on early return:
if messages.is_empty() {
    utils::save_output(b"{\"error\": \"empty input\"}");
    return;
}
```

See `fixes/safe_patterns.rs` for a full template with safe output handling.

---

### E2 — Output is garbled / unexpected format

The raw LLM response bytes are being saved directly. This is fine — the output format depends on which model was called:

- **UOMI model (model 1):** `{"response": "...", "time_taken": 1.2, ...}`
- **OpenAI-compatible model:** `{"choices": [{"message": {"content": "..."}}], ...}`

If you need to extract just the text content, use:
```rust
fn extract_content(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if let Ok(j) = serde_json::from_str::<serde_json::Value>(&s) {
        if let Some(c) = j["choices"][0]["message"]["content"].as_str() { return c.to_string(); }
        if let Some(c) = j["response"].as_str() { return c.to_string(); }
    }
    s.to_string()
}
```

---

### E3 — `call_ai_service` returns unexpected content

The response is valid but not what you expected. Add logging to inspect:

```rust
let response_bytes = utils::call_ai_service(1, utils::prepare_request(&body));
utils::log(&format!("Raw response: {}", String::from_utf8_lossy(&response_bytes)));
utils::save_output(&response_bytes);
```

Then run `npm start` and check the `[WASM]` log line in the terminal.

---

### E4 — IPFS fetch returns empty or hangs

```rust
let bytes = utils::get_cid_file_service("bafkrei...".as_bytes().to_vec());
// bytes is empty
```

Possible causes:
1. **Wrong CID** — verify the CID exists: `curl https://ipfs.io/ipfs/<cid>`
2. **IPFS gateway slow/down** — change gateway in `uomi.config.json`:
   ```json
   "ipfs": { "gateway": "https://cloudflare-ipfs.com/ipfs", "timeout_ms": 15000 }
   ```
3. **CID not pinned** — content may have been garbage collected by the gateway
4. **Timeout** — increase `ipfs.timeout_ms`

---

## F — On-chain errors

### F1 — `getAgentOutput` returns `success: false`

The agent WASM panicked or `save_output` was never called during on-chain execution.

**Debug locally first:** reproduce the exact input that was sent on-chain and run locally:
```bash
echo '<the inputData bytes as utf-8>' > host/src/input.txt
npm run build
cat host/src/output.txt
```

On-chain execution is identical to local execution — if it works locally, it will work on-chain.

---

### F2 — `callAgent` transaction reverts

Common causes:
- **Insufficient gas**: use `gas: 10_000_000` minimum
- **Wrong gas price**: use `gasPrice: Web3.utils.toWei('36.54', 'gwei')`
- **Wrong NFT ID**: verify the agent exists with `contract.methods.exists(nftId).call()`
- **Not enough value sent**: some agents require payment (check the `price` field when minted)

---

### F3 — `completed: false` after many blocks

The agent execution timed out on validators. Causes:
- Agent logic took too long (too many LLM calls, large IPFS fetches)
- Network congestion
- `minBlocks` set too low during minting

**Fix:** simplify the agent logic, or re-mint with a higher `minBlocks` value.

---

## Quick diagnostic checklist

Run through these if you're not sure where to start:

```bash
# 1. Is the WASM built?
ls -lh host/src/agent_template.wasm

# 2. Does the package name match?
cat agent-template/cargo.toml | grep "^name"
ls agent-template/target/wasm32-unknown-unknown/release/*.wasm 2>/dev/null

# 3. Is the wasm32 target installed?
rustup target list --installed | grep wasm32

# 4. Is the model config valid?
cat uomi.config.json

# 5. What did the agent last output?
cat host/src/output.txt 2>/dev/null || echo "(no output.txt)"

# 6. What does lib.rs look like?
cat agent-template/src/lib.rs
```
