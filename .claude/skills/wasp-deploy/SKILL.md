---
name: wasp-deploy
description: Deploy a UOMI WASM agent to the UOMI network. Covers building the final WASM and minting the agent as an ERC721 NFT on-chain. Use when a developer is ready to publish their agent.
argument-hint: [network: testnet|mainnet]
disable-model-invocation: true
allowed-tools: Read Write Bash(npm run build*) Bash(ls host/src/*.wasm*) Bash(cat uomi.config.json) Bash(node serve-deploy*)
---

# Deploy a UOMI Agent to the Network

You are helping a developer deploy their compiled WASM agent to the UOMI network.

Target network: **$ARGUMENTS** (default: testnet)

## Overview

Deploying a UOMI agent has two steps:
1. **Build the final WASM**
2. **Mint the agent NFT** via a local `deploy.html` page

**Exactly what happens — zero manual steps:**
1. Claude writes `deploy.html` and `serve-deploy.js`, then runs the server
2. The browser opens on `http://localhost:3333`
3. The developer clicks **Connect Wallet** — MetaMask connects and signs an auth message (no cost, just identity verification)
4. They select `host/src/agent_template.wasm`, fill in the metadata, click **Deploy**
5. The page uploads the WASM to `https://backend.uomi.ai/api/upload/wasm` using the auth token
6. MetaMask shows the `safeMint` transaction (costs **100 UOMI**)
7. They confirm — the page shows the `tokenId`

---

## Step 1 — Build the final WASM

Make sure `lib.rs` uses **model 1** before deploying:
```rust
let response = utils::call_ai_service(1, request);
```

```bash
npm run build
ls -lh host/src/agent_template.wasm
```

---

## Step 2 — Write deploy.html

Write this to the project root:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Deploy UOMI Agent</title>
  <script src="https://cdn.jsdelivr.net/npm/ethers@6.13.0/dist/ethers.umd.min.js"></script>
  <style>
    body { font-family: monospace; max-width: 640px; margin: 40px auto; padding: 0 20px; background: #0f0f0f; color: #e0e0e0; }
    h2 { color: #fff; }
    label { display: block; margin-top: 12px; font-size: 12px; color: #aaa; }
    input, textarea { width: 100%; padding: 8px; box-sizing: border-box; background: #1a1a1a; border: 1px solid #333; color: #e0e0e0; margin-top: 4px; font-family: monospace; }
    button { margin-top: 16px; padding: 12px 24px; background: #6c3de0; color: #fff; border: none; cursor: pointer; font-size: 14px; margin-right: 8px; }
    button:disabled { background: #444; cursor: not-allowed; }
    #wallet-status { margin-top: 12px; font-size: 12px; color: #aaa; }
    #log { margin-top: 20px; background: #1a1a1a; padding: 12px; white-space: pre-wrap; font-size: 12px; min-height: 60px; border: 1px solid #333; }
  </style>
</head>
<body>
  <h2>Deploy UOMI Agent</h2>

  <button id="connectBtn" onclick="connectWallet()">Connect Wallet</button>
  <div id="wallet-status">Not connected</div>

  <label>WASM File</label>
  <input id="wasmFile" type="file" accept=".wasm" />

  <label>Name</label>
  <input id="name" value="My UOMI Agent" />

  <label>Description</label>
  <input id="description" value="A UOMI agent" />

  <label>Input Schema (JSON)</label>
  <textarea id="inputSchema" rows="2">{"type":"array","items":{"role":"string","content":"string"}}</textarea>

  <label>Output Schema (JSON)</label>
  <textarea id="outputSchema" rows="2">{"type":"string"}</textarea>

  <label>Tags (comma-separated)</label>
  <input id="tags" value="chat,ai" />

  <label>Price in UOMI for callers (usually 0)</label>
  <input id="price" type="number" value="0" step="0.001" />

  <label>Min Validators</label>
  <input id="minValidators" type="number" value="1" />

  <label>Min Blocks</label>
  <input id="minBlocks" type="number" value="10" />

  <button id="deployBtn" onclick="deploy()" disabled>Deploy (100 UOMI)</button>

  <div id="log">Connect your wallet to start.</div>

  <script>
    const CONTRACT = '0xDb8434F12f21a678F749cb34E6CE0c168776461c';
    const CHAIN_ID = 4386;
    const RPC = 'https://rpc.testnet.uomi.network';

    const ABI = [{
      inputs: [{
        components: [
          { internalType: 'string',   name: 'name',          type: 'string'   },
          { internalType: 'string',   name: 'description',   type: 'string'   },
          { internalType: 'string',   name: 'inputSchema',   type: 'string'   },
          { internalType: 'string',   name: 'outputSchema',  type: 'string'   },
          { internalType: 'string[]', name: 'tags',          type: 'string[]' },
          { internalType: 'uint256',  name: 'price',         type: 'uint256'  },
          { internalType: 'uint256',  name: 'minValidators', type: 'uint256'  },
          { internalType: 'uint256',  name: 'minBlocks',     type: 'uint256'  },
          { internalType: 'string',   name: 'agentCID',      type: 'string'   },
        ],
        internalType: 'struct UomiAgent.Agent', name: 'agent', type: 'tuple'
      }, {
        internalType: 'address', name: 'to', type: 'address'
      }],
      name: 'safeMint',
      outputs: [],
      stateMutability: 'payable',
      type: 'function'
    }, {
      type: 'event', name: 'Transfer',
      inputs: [
        { name: 'from',    type: 'address', indexed: true },
        { name: 'to',      type: 'address', indexed: true },
        { name: 'tokenId', type: 'uint256', indexed: true },
      ]
    }];

    let authToken = null;
    let connectedAddress = null;

    const log = msg => { document.getElementById('log').textContent += '\n' + msg; };

    async function connectWallet() {
      const btn = document.getElementById('connectBtn');
      btn.disabled = true;
      document.getElementById('log').textContent = '';
      try {
        if (!window.ethereum) throw new Error('MetaMask not found');
        const provider = new ethers.BrowserProvider(window.ethereum);
        await provider.send('eth_requestAccounts', []);

        const network = await provider.getNetwork();
        if (Number(network.chainId) !== CHAIN_ID) {
          await window.ethereum.request({
            method: 'wallet_addEthereumChain',
            params: [{ chainId: '0x' + CHAIN_ID.toString(16), chainName: 'UOMI Testnet', rpcUrls: [RPC] }]
          });
        }

        const signer = await provider.getSigner();
        connectedAddress = await signer.getAddress();

        log('Fetching auth nonce...');
        const nonceRes = await fetch(`https://backend.uomi.ai/api/auth/nonce?address=${connectedAddress}`);
        const { nonce, timestamp } = await nonceRes.json();

        const message = `Welcome to UOMI Network\n\nI authorize this wallet to access the UOMI Network Dashboard and interact with AI Agents on the Layer 1 chain.\n\nThis signature is only used for authentication and will not trigger any blockchain transaction or incur any costs.\n\nDomain: dashboard.uomi.ai\nWallet address: ${connectedAddress}\nNonce: ${nonce}\nTimestamp: ${timestamp}`;

        log('Sign the auth message in MetaMask (no cost)...');
        const signature = await signer.signMessage(message);

        const authRes = await fetch('https://backend.uomi.ai/api/auth/verify', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ address: connectedAddress, signature, nonce }),
        });
        if (!authRes.ok) throw new Error('Authentication failed');
        const { token } = await authRes.json();
        authToken = token;

        document.getElementById('wallet-status').textContent = `Connected: ${connectedAddress}`;
        document.getElementById('deployBtn').disabled = false;
        btn.textContent = 'Reconnect';
        log('✅ Authenticated. Ready to deploy.');
      } catch (e) {
        log('❌ ' + (e.reason ?? e.message));
      } finally {
        btn.disabled = false;
      }
    }

    async function deploy() {
      const btn = document.getElementById('deployBtn');
      btn.disabled = true;
      document.getElementById('log').textContent = '';
      try {
        if (!authToken) throw new Error('Connect wallet first');

        const fileInput = document.getElementById('wasmFile');
        if (!fileInput.files.length) throw new Error('Select a .wasm file');
        const file = fileInput.files[0];

        const provider = new ethers.BrowserProvider(window.ethereum);
        const signer = await provider.getSigner();
        const contract = new ethers.Contract(CONTRACT, ABI, signer);

        log('Checking balance...');
        const balance = await provider.getBalance(connectedAddress);
        const required = ethers.parseEther('100');
        if (balance < required) {
          throw new Error(`Insufficient balance: ${ethers.formatEther(balance)} UOMI (need 100)`);
        }
        log(`Balance: ${ethers.formatEther(balance)} UOMI ✓`);

        log('Uploading WASM...');
        const formData = new FormData();
        formData.append('file', file);
        formData.append('name', document.getElementById('name').value);

        const uploadRes = await fetch('https://backend.uomi.ai/api/upload/wasm', {
          method: 'POST',
          headers: { Authorization: `Bearer ${authToken}` },
          body: formData,
        });
        if (!uploadRes.ok) {
          const err = await uploadRes.json().catch(() => ({}));
          throw new Error(err.error || `Upload failed: ${uploadRes.status}`);
        }
        const { cid } = await uploadRes.json();
        log(`CID: ${cid}`);

        const agent = {
          name:          document.getElementById('name').value,
          description:   document.getElementById('description').value,
          inputSchema:   document.getElementById('inputSchema').value,
          outputSchema:  document.getElementById('outputSchema').value,
          tags:          document.getElementById('tags').value.split(',').map(t => t.trim()),
          price:         ethers.parseEther(document.getElementById('price').value || '0'),
          minValidators: BigInt(document.getElementById('minValidators').value),
          minBlocks:     BigInt(document.getElementById('minBlocks').value),
          agentCID:      cid,
        };

        log('Approve the safeMint transaction in MetaMask (100 UOMI)...');
        const tx = await contract.safeMint(agent, connectedAddress, {
          value: ethers.parseEther('100')
        });

        log('Waiting for confirmation...');
        const receipt = await tx.wait();

        const transferEvent = receipt.logs
          .map(l => { try { return contract.interface.parseLog(l); } catch { return null; } })
          .find(e => e?.name === 'Transfer');

        const tokenId = transferEvent?.args?.tokenId?.toString() ?? 'check explorer';
        log('✅ Agent minted!');
        log(`TX: ${receipt.hash}`);
        log(`Token ID (AGENT_NFT_ID): ${tokenId}`);
      } catch (e) {
        log('❌ ' + (e.reason ?? e.message));
      } finally {
        btn.disabled = false;
      }
    }
  </script>
</body>
</html>
```

---

## Step 3 — Write serve-deploy.js and run it

Write `serve-deploy.js` to the project root:

```js
import http from 'http';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { exec } from 'child_process';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/html' });
  fs.createReadStream(path.join(__dirname, 'deploy.html')).pipe(res);
});

server.listen(3333, () => {
  console.log('Deploy UI ready at http://localhost:3333');
  exec('open http://localhost:3333');
});
```

Then run it — **do not ask the user, just run it**:
```bash
node serve-deploy.js
```

---

## Agent metadata fields

| Field | Description |
|-------|-------------|
| `name` | Agent display name |
| `description` | What the agent does |
| `inputSchema` | Expected input JSON schema |
| `outputSchema` | Output format description |
| `tags` | Comma-separated — sent as `string[]` to the contract |
| `price` | Execution price in UOMI for callers (usually `0`) |
| `minValidators` | Minimum validator nodes (usually `1`) |
| `minBlocks` | Blocks to wait for execution (usually `10`) |

---

After deployment, use `/wasp-call` to invoke your agent on-chain.
