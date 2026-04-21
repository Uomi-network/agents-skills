# UOMI Skills for Claude Code

A collection of [Claude Code](https://claude.ai/code) skills for building, testing, and deploying UOMI WASM agents.

## Skills

| Skill | What it does |
|-------|-------------|
| `/wasp` | **Orchestrator** — guides you through the full agent lifecycle end-to-end |
| `/wasp-create` | Scaffold a new UOMI agent project |
| `/wasp-agent` | Write or modify the Rust agent logic (`lib.rs`) |
| `/wasp-build` | Compile the Rust agent to WASM |
| `/wasp-dev` | Test the agent locally with an interactive console |
| `/wasp-deploy` | Deploy the WASM on-chain (IPFS upload + NFT mint via MetaMask) |
| `/wasp-proxy` | Build and deploy the Node.js backend (Railway, Render, Fly.io) |
| `/wasp-call` | Call an on-chain agent and read its output |
| `/wasp-debug` | Diagnose errors across the full stack |

---

## Installation

### Desktop (Claude Code app)

1. Open **Customize → Skills** in the left sidebar
2. Click **+** next to "Personal plugins" → **Add marketplace**
3. Enter `Uomi-network/agents-skills`
4. Browse the skills and enable the ones you want

### CLI

```bash
claude plugin marketplace add Uomi-network/uomi-skills
claude plugin install wasp@uomi-skills
```

Restart the session after installing.

### Verify

Open any project in Claude Code and type `/wasp` — you should see the skill in the autocomplete.

---

## Usage

The easiest entry point is the orchestrator:

```
/wasp build a grid trading bot
```

Claude will ask what you want to build, guide you through scaffolding, writing the Rust logic, testing, and deploying — invoking the sub-skills automatically.

You can also use skills individually:

```
/wasp-create my-agent ~/projects
/wasp-agent  
/wasp-build
/wasp-deploy
```

---

## Requirements

- [Claude Code](https://claude.ai/code)
- [Rust](https://rustup.rs) + `wasm32-unknown-unknown` target
- Node.js >= 14
- MetaMask (for deploy)
