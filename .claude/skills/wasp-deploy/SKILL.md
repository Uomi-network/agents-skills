---
name: wasp-deploy
description: Deploy a UOMI WASM agent to the UOMI network. Covers uploading WASM to IPFS and minting the agent as an ERC721 NFT on-chain via a MetaMask-compatible browser UI. Use when a developer is ready to publish their agent.
argument-hint: [network: testnet|mainnet]
disable-model-invocation: true
allowed-tools: Read Write Bash(npm run build*) Bash(ls host/src/*.wasm*) Bash(cat uomi.config.json)
---

# Deploy a UOMI Agent to the Network

You are helping a developer deploy their compiled WASM agent to the UOMI network.

Target network: **$ARGUMENTS** (default: testnet)

## Overview

Deploying a UOMI agent is a two-step process:
1. **Upload WASM to IPFS** — get a Content ID (CID)
2. **Mint agent NFT** — register the CID on-chain via `safeMint()`

**Exactly what happens:**
- The developer uploads the WASM to IPFS via Pinata (CLI or API) and gets a CID
- They open a local HTML page in their browser, fill in the agent metadata and CID
- They click "Mint" — MetaMask (or any injected wallet) pops up showing the transaction details
- They confirm in MetaMask — the transaction is sent and signed by their wallet
- The page shows the resulting `tokenId` — the agent's permanent on-chain ID
- No private keys are ever typed or stored anywhere

---

## Step 1 — Build the final WASM

Make sure you're using **model 1** (UOMI network model) in `lib.rs` before deploying:
```rust
let response = utils::call_ai_service(1, request);
```

Build and verify:
```bash
npm run build
ls -lh host/src/agent_template.wasm
```

---

## Step 2 — Upload WASM to IPFS

Pin the WASM on IPFS using Pinata:

```bash
# With curl + Pinata API
curl -X POST https://api.pinata.cloud/pinning/pinFileToIPFS \
  -H "Authorization: Bearer <YOUR_PINATA_JWT>" \
  -F "file=@host/src/agent_template.wasm" \
  | python3 -m json.tool
```

Copy the `IpfsHash` from the response — this is your CID.

---

## Step 3 — Create the mint UI

Create `deploy.html` in the project root:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Deploy UOMI Agent</title>
  <script src="https://cdn.jsdelivr.net/npm/ethers@6.13.0/dist/ethers.umd.min.js"></script>
  <style>
    body { font-family: monospace; max-width: 600px; margin: 40px auto; padding: 0 20px; }
    input, textarea { width: 100%; margin: 4px 0 12px; padding: 6px; box-sizing: border-box; }
    button { padding: 10px 20px; cursor: pointer; }
    #status { margin-top: 20px; white-space: pre-wrap; }
  </style>
</head>
<body>
  <h2>Deploy UOMI Agent</h2>

  <label>Agent Name</label>
  <input id="name" value="My UOMI Agent" />

  <label>Description</label>
  <input id="description" value="A UOMI agent" />

  <label>WASM CID (from IPFS)</label>
  <input id="cid" placeholder="bafkrei..." />

  <label>Input Schema (JSON)</label>
  <textarea id="inputSchema" rows="2">{"type":"array","items":{"role":"string","content":"string"}}</textarea>

  <label>Output Schema (JSON)</label>
  <textarea id="outputSchema" rows="2">{"type":"string"}</textarea>

  <label>Tags (comma-separated)</label>
  <input id="tags" value="chat,ai" />

  <label>Min Validators</label>
  <input id="minValidators" type="number" value="1" />

  <label>Min Blocks</label>
  <input id="minBlocks" type="number" value="10" />

  <button onclick="mint()">Connect wallet & Mint</button>

  <div id="status"></div>

  <script>
    const CONTRACT = '0xDb8434F12f21a678F749cb34E6CE0c168776461c';
    const RPC = 'https://rpc.testnet.uomi.network';
    const CHAIN_ID = 4386; // UOMI testnet

    const ABI = [{
      type: 'function',
      name: 'safeMint',
      inputs: [
        { name: 'agent', type: 'tuple', components: [
          { name: 'name',           type: 'string' },
          { name: 'description',    type: 'string' },
          { name: 'inputSchema',    type: 'string' },
          { name: 'outputSchema',   type: 'string' },
          { name: 'tags',           type: 'string' },
          { name: 'price',          type: 'uint256' },
          { name: 'minValidators',  type: 'uint256' },
          { name: 'minBlocks',      type: 'uint256' },
          { name: 'agentCid',       type: 'string' },
        ]},
        { name: 'to', type: 'address' }
      ],
      stateMutability: 'payable'
    }, {
      type: 'event',
      name: 'Transfer',
      inputs: [
        { name: 'from',    type: 'address', indexed: true },
        { name: 'to',      type: 'address', indexed: true },
        { name: 'tokenId', type: 'uint256', indexed: true },
      ]
    }];

    function log(msg) {
      document.getElementById('status').textContent += msg + '\n';
    }

    async function mint() {
      document.getElementById('status').textContent = '';
      try {
        if (!window.ethereum) throw new Error('No wallet detected. Install MetaMask.');

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
        const address = await signer.getAddress();
        const contract = new ethers.Contract(CONTRACT, ABI, signer);

        const agent = {
          name:          document.getElementById('name').value,
          description:   document.getElementById('description').value,
          inputSchema:   document.getElementById('inputSchema').value,
          outputSchema:  document.getElementById('outputSchema').value,
          tags:          document.getElementById('tags').value,
          price:         0n,
          minValidators: BigInt(document.getElementById('minValidators').value),
          minBlocks:     BigInt(document.getElementById('minBlocks').value),
          agentCid:      document.getElementById('cid').value,
        };

        log('Sending transaction — check your wallet...');
        const tx = await contract.safeMint(agent, address, {
          value: ethers.parseEther('10')
        });

        log('Waiting for confirmation...');
        const receipt = await tx.wait();

        const transferEvent = receipt.logs
          .map(l => { try { return contract.interface.parseLog(l); } catch { return null; } })
          .find(e => e?.name === 'Transfer');

        const tokenId = transferEvent?.args?.tokenId?.toString() ?? 'unknown';
        log('✅ Agent minted!');
        log('Transaction: ' + receipt.hash);
        log('Token ID (AGENT_NFT_ID): ' + tokenId);
      } catch (e) {
        log('❌ Error: ' + (e.reason ?? e.message));
      }
    }
  </script>
</body>
</html>
```

Open it directly in the browser:
```bash
open deploy.html
```

Fill in the form, click **Connect wallet & Mint**, and approve the transaction in MetaMask. The page will show the `tokenId` once confirmed.

---

## Step 4 — Verify deployment

Check the agent is on-chain using the UOMI explorer:
- Testnet: https://explorer.testnet.uomi.network

---

## Agent metadata fields

| Field | Description | Example |
|-------|-------------|---------|
| `name` | Agent display name | `"Weather Bot"` |
| `description` | What the agent does | `"Answers weather questions"` |
| `inputSchema` | Expected input JSON schema | `{"type":"array","items":{...}}` |
| `outputSchema` | Output format description | `{"type":"string"}` |
| `tags` | Comma-separated tags | `"weather,ai,chat"` |
| `minValidators` | Minimum validator nodes | `1` |
| `minBlocks` | Blocks to wait for execution | `10` |
| `agentCid` | IPFS CID of the WASM file | `"bafkrei..."` |

---

After deployment, use `/wasp-call` to invoke your agent on-chain.
