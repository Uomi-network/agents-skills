---
name: wasp-proxy
description: Build and deploy the backend component of a UOMI agent project. Every agent has two parts — the Rust WASM on-chain logic, and a backend server that uses UomiWeb2ProxySdk to invoke it. Use when a developer needs to create, configure, or deploy this backend layer.
argument-hint: [what API the backend should expose]
allowed-tools: Read Write Edit Bash(npm install*) Bash(npm init*) Bash(node --version*) Bash(ls*) Bash(cat*)
---

# UOMI Agent Backend

You are helping a developer build and deploy the **backend component** of their UOMI agent.

**What the backend should expose:** $ARGUMENTS

---

## The two-component architecture

Every UOMI agent project has two parts:

```
┌─────────────────────────────────────────────────┐
│  1. Rust WASM Agent  (agent-template/src/lib.rs) │
│     → compiled → deployed on-chain as NFT        │
│     → see /wasp-agent, /wasp-deploy              │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  2. Backend Server   (this skill)                │
│     → Node.js server                            │
│     → uses UomiWeb2ProxySdk to call the agent   │
│     → exposes any API you want to clients        │
└─────────────────────────────────────────────────┘
```

The backend is your web server: it can be a REST API, OpenAI-compatible proxy, webhook handler, Telegram bot, Discord bot — anything. The only common denominator is that it uses `UomiWeb2ProxySdk` to invoke the on-chain agent and get the result.

---

## The SDK: UomiWeb2ProxySdk

The SDK handles all the complexity of calling an on-chain agent: submitting the blockchain transaction, polling for the `requestId`, waiting for execution to complete, and returning the output.

```js
const UomiWeb2ProxySdk = require('./UomiWeb2ProxySdk');

const sdk = new UomiWeb2ProxySdk(
  process.env.PROXY_API_KEY,  // API key for the UOMI Web2 Proxy service
  process.env.PROXY_URL       // URL of the proxy (default: http://localhost:3000)
);
```

### `sdk.executeAgent(nftId, inputData, options)` — main method

Executes an agent end-to-end and returns the completed result.

```js
const result = await sdk.executeAgent(
  nftId,      // number — the agent's NFT tokenId (from /wasp-deploy)
  inputData,  // string — the input to pass to the agent (JSON stringified)
  {
    timeout:              60000,  // ms to wait before giving up (default: 30000)
    timelog:              false,  // log elapsed time at each poll step
    debug:                false,  // log raw responses
    requestIdRecoveryMs:  60000,  // extra window if tx sent but requestId not yet available
  }
);
```

**`inputData`** is a JSON string. For a chat agent, it's the messages array:
```js
const inputData = JSON.stringify([
  { role: 'system', content: 'You are a helpful assistant.' },
  { role: 'user',   content: userMessage }
]);
```

**Return value:**
```js
{
  requestId: 42,          // on-chain request ID
  output: '0x...',        // raw hex-encoded agent output bytes
  completed: true,
  // ...other chain fields
}
```

### Decoding the output

The agent output is hex-encoded bytes. Decode with:

```js
function decodeAgentOutput(output) {
  // Decode hex → string
  const raw = (typeof output === 'string' && output.startsWith('0x'))
    ? Buffer.from(output.slice(2), 'hex').toString('utf8')
    : String(output);

  // Try to extract text content from JSON response formats
  try {
    const parsed = JSON.parse(raw);
    return parsed.content || parsed.response || raw;
  } catch {
    return raw;
  }
}
```

This handles both UOMI model format (`{ "response": "..." }`) and OpenAI format (`{ "choices": [{ "message": { "content": "..." } }] }`).

### Lower-level methods

```js
// Check chain/proxy status
const chainInfo = await sdk.getChain();

// Get a specific request by ID
const req = await sdk.getRequest(requestId);

// Get the most recently completed request
const last = await sdk.getLastRequest();
```

---

## Step 1 — Set up the backend project

In the agent project root (alongside `agent-template/`), create the backend:

```bash
mkdir backend
cd backend
npm init -y
npm install express dotenv
```

Copy `UomiWeb2ProxySdk.js` into the backend:
```bash
cp path/to/UomiWeb2ProxySdk.js ./
```

---

## Step 2 — Configure the environment

Create `backend/.env`:

```env
PORT=3000

# UOMI Web2 Proxy — handles blockchain tx on your behalf
PROXY_URL=https://turing-a-w2p.uomi.ai
PROXY_API_KEY=<get from UOMI team>

# Your deployed agent
AGENT_NFT_ID=<tokenId from /wasp-deploy>
```

---

## Step 3 — Implement the server

The structure depends on what API you want to expose. Below are the main patterns.

### Pattern A: Minimal REST API

```js
// server.js
const express = require('express');
const UomiWeb2ProxySdk = require('./UomiWeb2ProxySdk');
require('dotenv').config();

const app   = express();
const sdk   = new UomiWeb2ProxySdk(process.env.PROXY_API_KEY, process.env.PROXY_URL);
const NFT   = parseInt(process.env.AGENT_NFT_ID);

app.use(express.json());

app.get('/health', async (req, res) => {
  const chain = await sdk.getChain();
  res.json({ status: 'ok', chain });
});

app.post('/ask', async (req, res) => {
  const { message } = req.body;
  if (!message) return res.status(400).json({ error: 'message required' });

  const inputData = JSON.stringify([{ role: 'user', content: message }]);

  const result = await sdk.executeAgent(NFT, inputData, { timeout: 60000 });
  if (!result) return res.status(500).json({ error: 'Agent execution failed or timed out' });

  res.json({ answer: decodeAgentOutput(result.output) });
});

function decodeAgentOutput(output) {
  const raw = (typeof output === 'string' && output.startsWith('0x'))
    ? Buffer.from(output.slice(2), 'hex').toString('utf8')
    : String(output);
  try {
    const p = JSON.parse(raw);
    return p.content || p.response || raw;
  } catch { return raw; }
}

app.listen(process.env.PORT || 3000, () =>
  console.log(`Backend running on :${process.env.PORT || 3000}`)
);
```

### Pattern B: OpenAI-compatible API

Exposes `POST /v1/chat/completions` so any OpenAI SDK can use it as a drop-in provider.

```js
app.post('/v1/chat/completions', async (req, res) => {
  const { messages, stream } = req.body;
  if (!messages) return res.status(400).json({ error: 'messages required' });

  const inputData = JSON.stringify(messages);
  const result = await sdk.executeAgent(NFT, inputData, { timeout: 120000 });
  if (!result) return res.status(500).json({ error: { message: 'Agent timed out' } });

  const content = decodeAgentOutput(result.output);
  const id      = `chatcmpl-${result.requestId || Date.now()}`;
  const created = Math.floor(Date.now() / 1000);

  if (stream) {
    res.setHeader('Content-Type', 'text/event-stream');
    res.setHeader('Cache-Control', 'no-cache');
    res.write(`data: ${JSON.stringify({ id, object: 'chat.completion.chunk', created, model: 'uomi', choices: [{ index: 0, delta: { role: 'assistant', content }, finish_reason: null }] })}\n\n`);
    res.write(`data: ${JSON.stringify({ id, object: 'chat.completion.chunk', created, model: 'uomi', choices: [{ index: 0, delta: {}, finish_reason: 'stop' }] })}\n\n`);
    res.write('data: [DONE]\n\n');
    return res.end();
  }

  res.json({
    id, object: 'chat.completion', created, model: 'uomi',
    choices: [{ index: 0, message: { role: 'assistant', content }, finish_reason: 'stop' }],
    usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }
  });
});
```

### Pattern C: Async (fire-and-forget)

When execution takes too long to wait synchronously:

```js
// POST /ask — enqueue, return immediately
app.post('/ask', async (req, res) => {
  const { message, webhookUrl } = req.body;
  const inputData = JSON.stringify([{ role: 'user', content: message }]);

  // Start execution without awaiting
  sdk.executeAgent(NFT, inputData, { timeout: 300000 })
    .then(result => {
      if (webhookUrl) {
        fetch(webhookUrl, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ answer: decodeAgentOutput(result.output) })
        });
      }
    });

  res.json({ status: 'processing' });
});
```

---

## Step 4 — Run locally

```bash
node server.js
```

Test:
```bash
curl -X POST http://localhost:3000/ask \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello!"}'
```

---

## Step 5 — Deploy to production

### VPS with PM2

```bash
npm install -g pm2
pm2 start server.js --name agent-backend
pm2 save && pm2 startup
```

### Docker

```dockerfile
FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --production
COPY . .
EXPOSE 3000
CMD ["node", "server.js"]
```

### Nginx (if streaming is needed)

```nginx
location / {
    proxy_pass http://localhost:3000;
    proxy_http_version 1.1;
    proxy_set_header Connection '';
    proxy_buffering off;   # required for SSE streaming
    proxy_cache off;
}
```

---

## Troubleshooting

**`executeAgent` returns `null`**
→ The agent timed out. Increase `timeout` option. Check the NFT ID and that the agent is deployed and functional with `/wasp-call`.

**`PROXY_API_KEY` auth error**
→ Verify the key against the UOMI Web2 Proxy. Test directly:
```bash
curl -H "x-api-key: $PROXY_API_KEY" $PROXY_URL/api/chain
```

**Output is empty string or `""`**
→ The agent's `run()` called `save_output` with empty data, or the output hex decodes to empty. Debug the agent with `/wasp-debug`.

**`requestId` recovery logs appearing**
→ The blockchain tx was submitted but the requestId took longer than expected to be indexed. This is normal — the SDK recovers automatically. Increase `requestIdRecoveryMs` if it keeps failing.
