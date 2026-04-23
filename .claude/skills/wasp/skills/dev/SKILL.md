---
name: wasp-dev
description: Run a UOMI agent in local development/test mode with an interactive console. Use when testing agent behavior, debugging responses, checking performance metrics, or iterating on agent logic.
allowed-tools: Bash(npm start*) Bash(npm run dev*) Bash(cat*) Bash(ls*)
---

# Run the UOMI Agent in Development Mode

You are helping a developer test their UOMI agent locally using the interactive console.

## How it works

`npm start` builds the WASM and opens an interactive console where you can:
- Send multi-turn messages to the agent
- See the agent's LLM responses
- Monitor performance metrics (tokens, latency)
- Use built-in dev commands

## Step 1 — Configure the model for local dev

For fast iteration without a UOMI node, use a third-party LLM. Edit `uomi.config.json`:

```json
{
  "local_file_path": "../agent-template/src/request_input_file_example.txt",
  "models": {
    "1": { "name": "Qwen/Qwen2.5-32B-Instruct-GPTQ-Int4" },
    "2": {
      "name": "gpt-4o-mini",
      "url": "https://api.openai.com/v1/chat/completions",
      "api_key": "sk-..."
    }
  }
}
```

Then in `agent-template/src/lib.rs`, call model `2` during dev:
```rust
let response = utils::call_ai_service(2, request); // 2 = OpenAI-compatible
```

Switch back to `1` before deploying to UOMI mainnet.

## Step 2 — Start the dev console

```bash
npm start
```

The console will rebuild and run on every invocation. You'll see:

```
UOMI Development Environment
Type your messages. Use these commands:
/clear   - Clear conversation history
/history - Show conversation history
/exit    - Exit the program

You: 
```

## Step 3 — Test your agent

### Basic conversation test
```
You: Hello, who are you?
Assistant: Hello! I'm your UOMI agent...

Performance Metrics:
- Time taken: 1.20s
- Tokens/second: 45
- Total tokens: 54
```

### Multi-turn test
The console maintains conversation history and injects it into each request as a JSON messages array — matching what `utils::parse_messages()` expects.

### Structured input test (non-chat agents)
For agents that use `read_input()` with a custom JSON struct (not chat messages), use `npm run build` instead of `npm start` — it runs once and exits:

```bash
echo '{"your":"input_json"}' > host/src/input.txt
npm run build
cat host/src/output.txt
```

The host reads from `host/src/input.txt` and writes to `host/src/output.txt`.

### Input file test
The `local_file_path` in `uomi.config.json` simulates the file input that `utils::get_input_file_service()` reads. Edit that file to test file-based inputs.

### IPFS CID test
`utils::get_cid_file_service(cid)` fetches content from IPFS. During dev this makes a real request to the configured IPFS gateway (`https://ipfs.io/ipfs` by default). Test with a known CID:
```rust
let cid = "bafkreicevizwv5glcsuhsqzokpowk4oh7kn4zl5xl5eiewjgfvxkhjgzdm".as_bytes().to_vec();
let content = utils::get_cid_file_service(cid);
utils::log(&format!("IPFS content: {:?}", String::from_utf8(content).unwrap()));
```

## Step 4 — Read performance metrics

| Metric | What it means |
|--------|---------------|
| Time taken | End-to-end response latency in seconds |
| Tokens/second | LLM inference speed (UOMI format only) |
| Total tokens | All tokens consumed in the response |
| Prompt/completion tokens | OpenAI format breakdown |

High latency or low tokens/second may indicate the model or node-ai is under load.

## Debugging tips

- Add `utils::log("message")` anywhere in `lib.rs` — logs print to the terminal during execution
- Use `/history` to see the full conversation being sent to the LLM
- Use `/clear` to reset context and test fresh
- Check `host/src/output.txt` — the raw agent output is written there after each run

## Dev loop

```
Edit lib.rs  →  npm start  →  test  →  /clear  →  repeat
```

Each `npm start` recompiles from scratch. Compilation takes ~10-30s depending on your machine.

Once the behavior is right, use `/wasp-deploy` to put the agent on-chain.
