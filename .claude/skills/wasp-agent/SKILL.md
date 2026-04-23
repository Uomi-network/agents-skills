---
name: wasp-agent
description: Help write, understand, or modify the Rust logic of a UOMI WASM agent. Use when a developer wants to build a specific type of agent, customize lib.rs, use UOMI host functions, or implement patterns like RAG, multi-step reasoning, or structured I/O.
argument-hint: "[what the agent should do]"
allowed-tools: Read Write Edit Bash(cat*) Bash(ls*) Bash(cargo test*) Bash(cargo build*)
---

# UOMI Agent — Rust Implementation Guide

You are helping a developer write or modify the Rust code for a UOMI WASM agent.

**What the developer wants to build:** $ARGUMENTS

---

## Your task

1. Ask clarifying questions if the goal is unclear (what input does it receive? what should it output? does it need IPFS, multi-step, structured data?)
2. Choose the right pattern (see below)
3. Write or modify `agent-template/src/lib.rs` directly — don't just explain, write the actual code
4. Explain any non-obvious choices

If the developer already has a `lib.rs`, read it first before suggesting changes.

---

## Architecture overview

```
On-chain call  →  host runtime  →  WASM (lib.rs)  →  host runtime  →  on-chain output
                  reads input.txt    run() function     writes output.txt
```

The agent lifecycle per invocation:
1. Host reads `input.txt` (the on-chain `inputData` bytes)
2. Host loads `agent_template.wasm` and calls `run()`
3. Inside `run()`, use the utils API to read input, call LLMs, fetch IPFS, write output
4. Host reads whatever `save_output()` wrote and returns it on-chain

**Everything is synchronous.** No async, no threads, no file system, no network (only via host functions).

---

## Full utils API reference

All functions live in `utils.rs` — import with `mod utils;` at the top of `lib.rs`.

### Input

```rust
// Read the raw input bytes (the inputData passed to callAgent on-chain)
pub fn read_input() -> Vec<u8>

// Parse the input as a JSON array of {role, content} messages
// Panics if input is not valid JSON messages array
pub fn parse_messages(input: &[u8]) -> Vec<Message>
```

### Messages helpers

```rust
// Create a system message
pub fn system_message(content: String) -> Message

// Prepend system message to a messages array
pub fn process_messages(system: Message, messages: Vec<Message>) -> Vec<Message>
```

### LLM calls

```rust
// Call an LLM model. model is the key in uomi.config.json (1, 2, 3, ...)
// content is the request body as bytes (use prepare_request to build it)
// Returns the raw LLM response bytes
pub fn call_ai_service(model: i32, content: Vec<u8>) -> Vec<u8>

// Convert a JSON string body to bytes ready for call_ai_service
pub fn prepare_request(body: &str) -> Vec<u8>
```

**Request body format:**
```rust
// Standard chat format
let body = format!("{{\"messages\": {}}}", serde_json::to_string(&messages).unwrap());
```

**Response format (UOMI model):**
```json
{ "response": "...", "time_taken": 1.2, "tokens_per_second": 45, "total_tokens_generated": 54 }
```

**Response format (OpenAI-compatible model):**
```json
{ "choices": [{ "message": { "content": "..." } }], "usage": { "total_tokens": 42 } }
```

### External data

```rust
// Fetch a file from IPFS by its CID
// cid is the CID string as bytes: "bafkrei...".as_bytes().to_vec()
pub fn get_cid_file_service(cid: Vec<u8>) -> Vec<u8>

// Read the input file (the inputCidFile parameter from callAgent on-chain,
// or the local_file_path from uomi.config.json during dev)
pub fn get_input_file_service() -> Vec<u8>
```

### Output and logging

```rust
// Write the agent output — this becomes the on-chain result
// Call this exactly once at the end of run()
pub fn save_output(data: &[u8])

// Print a debug message visible in the host terminal during local dev
pub fn log(message: &str)
```

---

## The Message type

Defined in `lib.rs` (not in utils):

```rust
#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,   // "system" | "user" | "assistant"
    content: String,
}
```

---

## Patterns

Reference implementations are in the `patterns/` directory next to this file.

### Pattern 1: Basic chat — `patterns/chat.rs`
The simplest agent. Adds a system prompt and calls the LLM.
Use when: building a custom chatbot, assistant, or any conversational agent.

```rust
let input = utils::read_input();
let messages = utils::parse_messages(&input);
let system = utils::system_message("You are ...".to_string());
let msgs = utils::process_messages(system, messages);
let body = format!("{{\"messages\": {}}}", serde_json::to_string(&msgs).unwrap());
let response = utils::call_ai_service(1, utils::prepare_request(&body));
utils::save_output(&response);
```

### Pattern 2: Structured input/output — `patterns/structured_input.rs`
Receives a custom JSON object, outputs a custom JSON object.
Use when: the caller sends structured data (not just chat messages) or you need a structured response.

Key additions:
- Define `#[derive(Deserialize)] struct AgentInput { ... }`
- Define `#[derive(Serialize)] struct AgentOutput { ... }`
- Parse with `serde_json::from_slice::<AgentInput>(&raw)`
- Save with `serde_json::to_string(&output).unwrap().as_bytes()`

### Pattern 3: RAG with IPFS — `patterns/ipfs_rag.rs`
Fetches a knowledge document from IPFS, injects it as context.
Use when: building a Q&A bot over a specific document, knowledge base, or dataset.

Key step:
```rust
let doc_bytes = utils::get_cid_file_service("bafkrei...".as_bytes().to_vec());
let doc = String::from_utf8_lossy(&doc_bytes).to_string();
// Inject doc into system prompt, truncate to ~12k chars to stay within limits
```

### Pattern 4: Multi-step reasoning — `patterns/multi_step.rs`
Calls the LLM multiple times. First to classify/plan, then to answer.
Use when: you need routing logic, chain-of-thought, or different prompts for different query types.

Key constraint: **each `call_ai_service` is a blocking call** — keep chains short (2-3 steps) to avoid timeouts.

---

## cargo.toml — adding dependencies

```toml
[package]
name = "your-agent-name"   # must match directory name (hyphens → underscores for WASM output)
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]    # required — produces a .wasm file

[dependencies]
serde      = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# Add pure-Rust crates here. Avoid crates that need OS, network, or threads.
```

**Works in WASM** (pure Rust, no OS deps):
- `serde` / `serde_json` — already included
- `base64` — encode/decode base64
- `hex` — hex encoding
- `sha2` / `sha3` / `md-5` — hashing
- `uuid` (with `v4` feature disabled) — UUID parsing
- `regex` — pattern matching
- `chrono` (with `wasmbind` feature) — date/time parsing

**Won't work in WASM** (need OS/network):
- `reqwest`, `hyper`, `tokio` — use `call_ai_service` instead
- `std::fs` — no file system
- `std::thread` — no threads
- `rand` with OS entropy — use a deterministic seed or a WASM-compatible RNG

---

## Common mistakes

### Panic on empty input
```rust
// ❌ panics if input is empty or malformed
let messages = utils::parse_messages(&input);

// ✅ handle gracefully
let messages = serde_json::from_slice::<Vec<Message>>(&input).unwrap_or_else(|e| {
    utils::log(&format!("Parse error: {}", e));
    vec![Message { role: "user".to_string(), content: String::from_utf8_lossy(&input).to_string() }]
});
```

### Forgetting save_output
```rust
// ❌ agent completes but returns nothing
let response = utils::call_ai_service(1, request);
// missing: utils::save_output(&response);

// ✅
utils::save_output(&response);
```

### Calling save_output multiple times
Only the **last** call to `save_output` wins (host overwrites). Call it once at the end.

### Token limits
MAX_INPUT_SIZE is 1MB. Keep injected documents under ~12,000 characters to avoid hitting model context limits.

---

## Parsing the LLM response in Rust

When you need to extract the text content from the LLM response:

```rust
fn extract_content(response_bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(response_bytes);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
        // OpenAI format
        if let Some(c) = json["choices"][0]["message"]["content"].as_str() {
            return c.to_string();
        }
        // UOMI format
        if let Some(c) = json["response"].as_str() {
            return c.to_string();
        }
    }
    s.to_string()
}
```

---

## Unit tests

Always write unit tests for the `decide` function (or any pure logic function).
Because `utils.rs` uses `extern "C"` host symbols that don't exist in a native binary,
you must gate out the host-dependent code during test builds.

**Required pattern — add these two `#[cfg(not(test))]` guards:**

```rust
// 1. Gate the utils module — prevents unresolved extern "C" link errors
#[cfg(not(test))]
mod utils;

// 2. Gate run() — it calls utils functions
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() { ... }

// 3. Gate save() helper if it calls utils::save_output
#[cfg(not(test))]
fn save(output: AgentOutput) {
    utils::save_output(serde_json::to_string(&output).unwrap().as_bytes());
}
```

Run tests on the **native** target (not wasm32):
```bash
# from agent-template/
cargo test
```

**Test module template:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(/* ... */) -> AgentInput { /* ... */ }

    #[test]
    fn test_basic_case() {
        let out = decide(&make_input(/* ... */));
        assert_eq!(out.action, "buy");
        assert!(out.reason.contains("expected text"));
    }
}
```

**Common pitfall — identity conditions:**
Conditions like `value >= value / n` are always true for `n >= 1`. Double-check
that your `can_buy`/`can_sell` guards actually gate on zero balance:
```rust
// ❌ always true
let can_buy = portfolio.quote >= buy_amount * buy_price; // = quote >= quote/levels

// ✅ actually guards zero balance
let can_buy = portfolio.quote > 0.0;
```

## Testing with real input (integration)

Set `local_file_path` in `uomi.config.json` to a file with your test input, then:

```bash
# from project root
echo '{"your":"input"}' > host/src/input.txt
npm run build
cat host/src/output.txt
```

Logs from `utils::log()` appear prefixed with `[WASM]` in the terminal.
