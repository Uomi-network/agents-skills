---
name: wasp
description: Orchestrator for building a complete UOMI agent end-to-end. Guides through the full workflow — scaffold project, write Rust logic, test, build, and optionally deploy on-chain and set up the backend proxy.
argument-hint: [what the agent should do]
disable-model-invocation: true
---

# UOMI Agent — Full Workflow Orchestrator

You are guiding a developer through the complete process of building a UOMI WASM agent.

**What to build:** $ARGUMENTS

**First**, ask the user what they want to build. Let them describe their idea freely.

Once you understand the idea, ask for the project name and destination path.

Then, based on the idea, explain whether a backend is needed and why:

**A backend is needed when:**
- The agent needs to be triggered by an external event (API call, webhook, schedule)
- The result needs to be delivered to a Web2 system
- The caller can't interact directly with the blockchain

**A backend is NOT needed when:**
- The agent is called directly on-chain by another smart contract
- The developer is just experimenting or testing

Important constraints to keep in mind when evaluating the idea:
- The agent runs deterministically on-chain — no internet access, no file system, no threads
- It cannot execute transactions autonomously
- It CAN call an LLM (non-deterministic, but supported)
- All external data must be passed in as input or fetched from IPFS

After explaining, ask the user to confirm whether they want the backend before proceeding.

Then proceed through the phases below.

---

## Phase 1 — Scaffold the project

Invoke the create skill to set up the project structure:

```
/wasp-create $ARGUMENTS
```

Wait for the project to be created before proceeding.

---

## Phase 2 — Write the agent logic

Once the project exists, invoke the agent skill to write `lib.rs`:

```
/wasp-agent $ARGUMENTS
```

This covers:
- Choosing the right pattern (chat, structured I/O, RAG, multi-step)
- Writing the full `run()` implementation
- Adding unit tests with `#[cfg(not(test))]` guards

---

## Phase 3 — Run unit tests

After writing the logic, run tests on the native target:

```bash
cd <project-name>/agent-template && cargo test
```

Fix any failing tests before proceeding. Common issues:
- Identity conditions that are always true (e.g. `x >= x/n`) — check your guards carefully
- Missing `#[cfg(not(test))]` on `mod utils` or `run()`

---

## Phase 4 — Build and test interactively

Invoke the build skill:

```
/wasp-build
```

Then test with the dev console:

```
/wasp-dev
```

If you hit errors at any point, `/wasp-debug` will diagnose them.

---

## Phase 5 — Deploy (manual step)

When the agent behaves correctly, deploy it on-chain:

```
/wasp-deploy
```

This will:
1. Upload the WASM to IPFS
2. Mint it as an NFT on the UOMI contract

> ⚠️ This is a blockchain transaction — review carefully before signing.

---

## Phase 6 — Set up the backend (optional)

If the user confirmed they need a backend, invoke:

```
/wasp-proxy
```

This sets up a Node.js service using `UomiWeb2ProxySdk` to call the on-chain agent and expose it via HTTP. If the user said no backend, skip this phase.

---

## Phases at a glance

| Phase | Skill | Reversible |
|-------|-------|-----------|
| Scaffold | `/wasp-create` | ✅ |
| Write logic | `/wasp-agent` | ✅ |
| Test | `cargo test` | ✅ |
| Build + dev | `/wasp-build`, `/wasp-dev` | ✅ |
| Deploy on-chain | `/wasp-deploy` | ⚠️ irreversible |
| Backend | `/wasp-proxy` | ✅ |

At any point: `/wasp-debug` to diagnose errors.
