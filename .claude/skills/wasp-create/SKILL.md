---
name: wasp-create
description: Scaffold a new UOMI agent project using WASP. Use when a developer wants to create a new UOMI agent, start a new agent project, or set up a WASM/Rust agent for the UOMI network.
argument-hint: [project-name] [destination-path]
allowed-tools: Bash(git clone*) Bash(mv *) Bash(cp -r *) Bash(ls*) Bash(mkdir*) Bash(npm install) Bash(chmod*) Bash(rustup target list*) Bash(rustup target add*) Bash(node --version) Bash(rustc --version) Bash(cargo --version)
---

# Create a new UOMI Agent project

You are helping a developer scaffold a new UOMI agent using WASP.

$ARGUMENTS

Parse `$ARGUMENTS` for:
- **project name** — directory name for the project (letters, numbers, hyphens only; default: `my-uomi-agent`)
- **destination path** — where to create the project (default: current directory)

## Step 1 — Check prerequisites

Run these checks in parallel:

```bash
node --version
rustc --version
cargo --version
rustup target list --installed | grep wasm32-unknown-unknown
```

If the wasm target is missing:
```bash
rustup target add wasm32-unknown-unknown
```

## Step 2 — Clone and set up

```bash
git clone https://github.com/Uomi-network/uomi-chat-agent-template.git <destination>/<project-name>
cd <destination>/<project-name>
npm install
chmod +x ./bin/build_and_run_host.sh
```

## Step 3 — Explain the project structure

After creation, show the developer the layout:

```
<project-name>/
├── agent-template/          # Your Rust WASM agent code
│   ├── cargo.toml           # Rust package config
│   └── src/
│       ├── lib.rs           # ← MAIN FILE: Your agent logic goes here
│       ├── utils.rs         # UOMI host function bindings (do not modify)
│       └── request_input_file_example.txt
├── host/                    # Rust host runtime (runs the WASM)
│   └── src/
│       └── agent_template.wasm  # Built output lands here (after build)
├── bin/
│   └── build_and_run_host.sh    # Build script
├── uomi.config.json         # LLM model configuration
├── main.js                  # Interactive dev console
└── package.json
```

**`uomi.config.json`** — configure which LLM to use:
```json
{
  "models": {
    "1": { "name": "Qwen/Qwen2.5-32B-Instruct-GPTQ-Int4" },
    "2": {
      "name": "gpt-3.5-turbo",
      "url": "https://api.openai.com/v1/chat/completions",
      "api_key": "YOUR_KEY"
    }
  }
}
```
Model `1` = UOMI network (no API key, requires node-ai). Model `2`+ = third-party LLMs for local dev.

## Step 4 — Next steps

```
Your agent is ready. Next:

  cd <project-name>/agent

  # To test immediately:
  npm start     # build + run interactive console

  # To write your agent logic:
  # Edit agent-template/src/lib.rs
```

Use `/wasp-agent` to write the agent logic, `/wasp-build` for build details, `/wasp-dev` for testing tips.
