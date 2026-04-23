---
name: wasp-call
description: Call a deployed UOMI agent on-chain and retrieve its output. Use when a developer wants to invoke an agent by its NFT ID, send input data, monitor execution, and read the result.
argument-hint: "[nft-id] [input-message]"
allowed-tools: Bash(node*) Bash(npm install web3*) Bash(cat*)
---

# Call a UOMI Agent On-Chain

You are helping a developer invoke a deployed UOMI agent and retrieve its output.

Agent NFT ID: **$ARGUMENTS**

## How agent execution works

1. You call `callAgent(nftId, inputCidFile, inputData)` on the contract
2. The UOMI network validators execute the agent's WASM with your input
3. The contract emits a `RequestSent` event with a `requestId`
4. Once validators complete execution, `getAgentOutput(requestId)` returns the result

Execution time depends on `minBlocks` set during agent minting and network load.

---

## Step 1 — Prepare your call script

Create `call-agent.js`:

```javascript
import { Web3 } from 'web3';

// --- CONFIG ---
const RPC_URL     = 'https://rpc.testnet.uomi.network';
const RPC_WS_URL  = 'wss://rpc.testnet.uomi.network';  // for event listening
const PRIVATE_KEY = process.env.PRIVATE_KEY;
const CONTRACT_ADDRESS = '0xDb8434F12f21a678F749cb34E6CE0c168776461c';

const NFT_ID      = process.argv[2] || '1';
const INPUT_MSG   = process.argv[3] || 'Hello!';

// Input must be a JSON array of messages (same format as WASP dev console)
const INPUT_DATA  = JSON.stringify([
  { role: 'user', content: INPUT_MSG }
]);

const ABI = [
  {
    "type": "function",
    "name": "callAgent",
    "inputs": [
      { "name": "nftId",         "type": "uint256" },
      { "name": "inputCidFile",  "type": "bytes"   },
      { "name": "inputData",     "type": "bytes"   }
    ],
    "stateMutability": "payable"
  },
  {
    "type": "function",
    "name": "getAgentOutput",
    "inputs": [{ "name": "_requestId", "type": "uint256" }],
    "outputs": [{
      "name": "",
      "type": "tuple",
      "components": [
        { "name": "output",    "type": "bytes"   },
        { "name": "completed", "type": "bool"    },
        { "name": "success",   "type": "bool"    }
      ]
    }],
    "stateMutability": "view"
  },
  {
    "type": "event",
    "name": "RequestSent",
    "inputs": [
      { "name": "requestId", "type": "uint256", "indexed": true },
      { "name": "nftId",     "type": "uint256", "indexed": true }
    ]
  }
];

async function main() {
  const web3 = new Web3(RPC_URL);
  const account = web3.eth.accounts.privateKeyToAccount(PRIVATE_KEY);
  web3.eth.accounts.wallet.add(account);

  const contract = new web3.eth.Contract(ABI, CONTRACT_ADDRESS);

  // Encode input: empty CID file, data as hex
  const inputCidFile = '0x';
  const inputData    = web3.utils.utf8ToHex(INPUT_DATA);

  console.log(`Calling agent #${NFT_ID}...`);
  console.log(`Input: ${INPUT_DATA}`);

  const tx = await contract.methods
    .callAgent(NFT_ID, inputCidFile, inputData)
    .send({
      from:     account.address,
      gas:      10_000_000,
      gasPrice: Web3.utils.toWei('36.54', 'gwei'),
    });

  console.log('Transaction:', tx.transactionHash);

  // Extract requestId from RequestSent event
  const requestId = tx.events?.RequestSent?.returnValues?.requestId;
  if (!requestId) {
    console.error('Could not find RequestSent event in receipt');
    console.log('Events:', JSON.stringify(tx.events, null, 2));
    return;
  }

  console.log(`Request ID: ${requestId}`);
  console.log('Waiting for agent execution...');

  // Poll for result
  await pollForOutput(contract, requestId);
}

async function pollForOutput(contract, requestId, maxAttempts = 30, intervalMs = 6000) {
  for (let i = 0; i < maxAttempts; i++) {
    const result = await contract.methods.getAgentOutput(requestId).call();

    if (result.completed) {
      if (result.success) {
        const output = Buffer.from(result.output.slice(2), 'hex').toString('utf-8');
        console.log('\n✅ Agent output:');
        console.log(output);
      } else {
        console.error('❌ Agent execution failed');
      }
      return;
    }

    process.stdout.write(`.`);
    await new Promise(r => setTimeout(r, intervalMs));
  }

  console.log(`\n⏰ Timed out after ${maxAttempts} attempts. Try polling manually:`);
  console.log(`node -e "
    import { Web3 } from 'web3';
    const w = new Web3('${RPC_URL}');
    const c = new w.eth.Contract(ABI, '${CONTRACT_ADDRESS}');
    c.methods.getAgentOutput(${requestId}).call().then(r => console.log(r));
  "`);
}

main().catch(console.error);
```

---

## Step 2 — Run the call

```bash
# Install web3 if not already present
npm install web3

# Call the agent
PRIVATE_KEY=0x... node call-agent.js <nft-id> "Your message here"

# Example:
PRIVATE_KEY=0x... node call-agent.js 1 "What is 2+2?"
```

---

## Step 3 — Understand the output

The agent output is the raw bytes written by `utils::save_output()` in the WASM.

For a standard chat agent this is the LLM response JSON:
```json
{
  "response": "2+2 equals 4",
  "time_taken": 1.23,
  "tokens_per_second": 45,
  "total_tokens_generated": 12
}
```

Or OpenAI format if using a third-party model:
```json
{
  "choices": [{ "message": { "content": "2+2 equals 4" } }],
  "usage": { "total_tokens": 42 }
}
```

---

## Calling with a file input (CID)

If your agent uses `utils::get_input_file_service()`, pass the IPFS CID as `inputCidFile`:

```javascript
// Convert CID string to bytes
const cid = 'bafkrei...';
const inputCidFile = web3.utils.utf8ToHex(cid);  // instead of '0x'
```

---

## Calling via the contract directly (Hardhat/Foundry)

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IUomiAgents {
    function callAgent(uint256 nftId, bytes calldata inputCidFile, bytes calldata inputData) external payable;
    function getAgentOutput(uint256 _requestId) external view returns (bytes memory output, bool completed, bool success);
}

contract AgentCaller {
    IUomiAgents constant AGENTS = IUomiAgents(0xDb8434F12f21a678F749cb34E6CE0c168776461c);

    function callMyAgent(uint256 nftId, string calldata message) external payable {
        bytes memory input = abi.encodePacked(
            '[{"role":"user","content":"', message, '"}]'
        );
        AGENTS.callAgent(nftId, bytes(""), input);
    }
}
```
