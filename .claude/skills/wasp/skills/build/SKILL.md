---
name: wasp-build
description: Build a UOMI WASM agent from Rust source code. Use when a developer wants to compile their agent, run the build pipeline, or troubleshoot Rust/WASM compilation errors.
allowed-tools: Bash(npm run build*) Bash(npm run*) Bash(cargo build*) Bash(sh ./bin/*) Bash(ls*) Bash(cat cargo.toml) Bash(rustup target list*)
---

# Build the UOMI WASM Agent

You are helping a developer compile their UOMI agent from Rust to WebAssembly.

## What the build does

The build pipeline (`npm run build` → `bin/build_and_run_host.sh`) performs:

1. **Compile Rust → WASM**: `cargo build --target wasm32-unknown-unknown --release` inside `agent-template/`
2. **Copy artifact**: copies the `.wasm` file to `host/src/agent_template.wasm`
3. **Run host**: `cd host && cargo run` — executes the agent against the configured input

## Step 1 — Verify you're in the right directory

Check you're in the project root (where `package.json` lives):

```bash
ls package.json agent-template/cargo.toml 2>/dev/null && echo "OK" || echo "Wrong directory — cd into your project root"
```

## Step 2 — Run the build

```bash
npm run build
```

This is equivalent to:
```bash
sh ./bin/build_and_run_host.sh
```

## Step 3 — Interpret the output

**Successful build looks like:**
```
   Compiling agent-template v0.1.0
    Finished release [optimized] target(s)
   Compiling host v0.1.0
    Finished dev [unoptimized + debuginfo] target(s)
     Running `target/debug/host`
```

**Successful agent execution looks like:**
```
Assistant:
<response from the LLM>

Performance Metrics:
- Time taken: 1.20s
- Tokens/second: 45
- Total tokens: 54
```

## Troubleshooting common errors

### `error[E0463]: can't find crate for 'std'` or `wasm32 target not found`
```bash
rustup target add wasm32-unknown-unknown
```

### `error: linker 'cc' not found` (Linux)
```bash
sudo apt-get install build-essential
```

### `error: package not found` in cargo.toml
The project name in `agent-template/cargo.toml` must match the directory name (underscores, not hyphens). Check:
```bash
cat agent-template/cargo.toml | grep "^name"
# Should match the directory name with - replaced by _
```

### Connection/API errors at runtime
The host is running your WASM and calling an LLM. Check `uomi.config.json`:
- Is `models.1.name` set? (uses UOMI network — requires node-ai service)
- For local dev, set `models.2` with a valid `url` + `api_key` and use `call_ai_service(2, request)` in your `lib.rs`

### `thread 'main' panicked` in WASM
Add debug logs in your `lib.rs` to trace the issue:
```rust
utils::log("checkpoint 1: input read");
let input = utils::read_input();
utils::log(&format!("input length: {}", input.len()));
```
Logs appear in the terminal output.

## After a successful build

The built WASM is at `host/src/agent_template.wasm`. This is the file you'll upload to IPFS when deploying.

To run interactively (multi-turn conversation), use `/wasp-dev` instead of `npm run build`.
