# Adding a New Blockchain Network

## Overview

This guide walks you through the process of adding support for a new blockchain network to the ERC-8004 backend infrastructure. The project currently supports 7 blockchain networks (4 testnets, 3 mainnets) and is designed to make adding new chains straightforward.

**Difficulty Level**: ⭐⭐⭐ EASY-MODERATE (3/5)

**Time Estimate**:
- Basic configuration: 30-45 minutes
- Complete testing: 30-60 minutes
- Documentation updates: 15-30 minutes
- **Total**: 1.5-2.5 hours for experienced developers

**Prerequisites**:
- ERC-8004 contracts deployed on target chain
- RPC provider access (Alchemy, Infura, QuickNode, or Ankr recommended)
- Basic understanding of Ponder indexer architecture
- Access to project repository

---

## Current Architecture

### Supported Networks

The project uses a **modular multi-chain architecture** with automatic chain detection:

**Testnets** (4):
- Ethereum Sepolia (chainId: 11155111)
- Base Sepolia (chainId: 84532)
- Linea Sepolia (chainId: 59141)
- Polygon Amoy (chainId: 80002)

**Mainnets** (3):
- Ethereum Mainnet (chainId: 1)
- Base Mainnet (chainId: 8453)
- Linea Mainnet (chainId: 59144)

### Why Adding Chains is Easy

✅ **Well-defined patterns**: Every chain follows the same configuration schema
✅ **Automatic validation**: Zod validates all environment variables at startup
✅ **Health check system**: Automatically tests RPC provider connectivity
✅ **Multi-provider failover**: Built-in support for 4+ RPC providers per chain
✅ **Dynamic configuration**: Chains are enabled automatically if properly configured
✅ **No database changes**: Event Store supports all chains without schema modifications

### Architecture Components

```
┌─────────────────────────────────────────────────────────────┐
│                   New Chain Configuration                    │
├─────────────────────────────────────────────────────────────┤
│ 1. ponder.config.ts        → Network definition             │
│ 2. env-validation.ts       → Zod schema validation          │
│ 3. .env                    → RPC URLs + contract addresses  │
│ 4. Event Store (PostgreSQL) → Automatic support (no changes)│
└─────────────────────────────────────────────────────────────┘
```

---

## Prerequisites

### 1. Smart Contract Deployment

You must have the **three ERC-8004 contracts** deployed on the target chain:

- **IdentityRegistry** - Agent registration and metadata
- **ReputationRegistry** - Feedback and reputation scores
- **ValidationRegistry** - Validation requests and responses

**Contract Addresses Required**:
- Identity Registry address (checksummed format)
- Reputation Registry address (checksummed format)
- Validation Registry address (checksummed format)
- Start block number (block where contracts were deployed)

**How to Get Addresses**:
```bash
# If contracts are already deployed, check the deployment repository
# Example: https://github.com/erc-8004/erc-8004-contracts

# Or deploy contracts yourself using Foundry/Hardhat
forge create --rpc-url $RPC_URL --private-key $PRIVATE_KEY \
  src/IdentityRegistry.sol:IdentityRegistry
```

### 2. RPC Provider Access

You need **at least one RPC provider** for the target chain. The project supports:

- **Alchemy** (recommended) - https://alchemy.com
- **Infura** - https://infura.io
- **QuickNode** - https://quicknode.com
- **Ankr** - https://ankr.com

**Best Practice**: Configure 2-3 providers for automatic failover.

### 3. Development Environment

Ensure you have:
- Node.js 20+ installed
- pnpm package manager
- Access to project `.env` file
- Text editor with TypeScript support

### 4. Chain Information

Gather the following information:

| Information | Example | Where to Find |
|-------------|---------|---------------|
| Chain Name | Optimism Sepolia | Chain documentation |
| Chain ID | 11155420 | https://chainlist.org |
| RPC URL | https://sepolia.optimism.io | Provider dashboard |
| Block Explorer | https://sepolia-optimism.etherscan.io | Chain documentation |
| Average Block Time | 2 seconds | Chain documentation |

---

## Step-by-Step Implementation

### Step 1: Update Ponder Configuration

**File**: `ponder-indexers/ponder.config.ts`

#### 1.1 Add Network Definition

Locate the `networks` object and add your chain:

```typescript
const networks: Record<string, NetworkConfig> = {
  ethereumSepolia: { chainId: 11155111, name: "Ethereum Sepolia" },
  baseSepolia: { chainId: 84532, name: "Base Sepolia" },
  lineaSepolia: { chainId: 59141, name: "Linea Sepolia" },
  polygonAmoy: { chainId: 80002, name: "Polygon Amoy" },
  ethereumMainnet: { chainId: 1, name: "Ethereum Mainnet" },
  baseMainnet: { chainId: 8453, name: "Base Mainnet" },
  lineaMainnet: { chainId: 59144, name: "Linea Mainnet" },

  // ADD YOUR CHAIN HERE
  optimismSepolia: {
    chainId: 11155420,
    name: "Optimism Sepolia",
  },
};
```

**Key Points**:
- Use camelCase for the network key (e.g., `optimismSepolia`)
- Provide accurate chainId (verify on https://chainlist.org)
- Use descriptive name for logging

#### 1.2 Add Contract Addresses

Locate the `contracts` object and add your deployed contract addresses:

```typescript
const contracts: Record<string, Record<string, `0x${string}`>> = {
  baseSepolia: {
    identity: getContractAddress("BASE_SEPOLIA_IDENTITY_ADDRESS"),
    reputation: getContractAddress("BASE_SEPOLIA_REPUTATION_ADDRESS"),
    validation: getContractAddress("BASE_SEPOLIA_VALIDATION_ADDRESS"),
  },
  // ... other chains

  // ADD YOUR CHAIN HERE
  optimismSepolia: {
    identity: getContractAddress("OPTIMISM_SEPOLIA_IDENTITY_ADDRESS"),
    reputation: getContractAddress("OPTIMISM_SEPOLIA_REPUTATION_ADDRESS"),
    validation: getContractAddress("OPTIMISM_SEPOLIA_VALIDATION_ADDRESS"),
  },
};
```

**Important**: The `getContractAddress()` helper reads from environment variables (see Step 3).

#### 1.3 Add Environment Prefix Mapping

Locate the `networkEnvPrefixes` object:

```typescript
const networkEnvPrefixes: Record<string, string> = {
  ethereumSepolia: "ETHEREUM_SEPOLIA",
  baseSepolia: "BASE_SEPOLIA",
  lineaSepolia: "LINEA_SEPOLIA",
  polygonAmoy: "POLYGON_AMOY",
  ethereumMainnet: "ETHEREUM_MAINNET",
  baseMainnet: "BASE_MAINNET",
  lineaMainnet: "LINEA_MAINNET",

  // ADD YOUR CHAIN HERE
  optimismSepolia: "OPTIMISM_SEPOLIA",
};
```

**Key Points**:
- Network key must match the key in `networks` object
- Prefix must be UPPER_SNAKE_CASE
- This prefix is used for all environment variables

---

### Step 2: Update Environment Validation

**File**: `ponder-indexers/src/env-validation.ts`

This file uses [Zod](https://zod.dev/) for runtime type-safe validation of environment variables.

#### 2.1 Add RPC Provider Schema

Locate the `rpcProviderSchema` object and add your chain's RPC URLs:

```typescript
const rpcProviderSchema = {
  // Ethereum Sepolia
  ETHEREUM_SEPOLIA_RPC_ALCHEMY: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_INFURA: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_QUIKNODE: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_ANKR: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_URL: httpsUrl.optional(),

  // ... other chains

  // ADD YOUR CHAIN HERE
  OPTIMISM_SEPOLIA_RPC_ALCHEMY: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_INFURA: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_QUIKNODE: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_ANKR: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_URL: httpsUrl.optional(), // Legacy fallback
};
```

**Key Points**:
- Use the prefix defined in Step 1.3 (`OPTIMISM_SEPOLIA`)
- Add entries for all 4 supported providers + generic fallback
- All entries are `optional()` - at least one must be provided
- `httpsUrl` validator ensures HTTPS-only URLs

#### 2.2 Add Contract Address Schema

Locate the `contractAddressSchema` object:

```typescript
const contractAddressSchema = {
  // Base Sepolia
  BASE_SEPOLIA_IDENTITY_ADDRESS: ethereumAddress.optional(),
  BASE_SEPOLIA_REPUTATION_ADDRESS: ethereumAddress.optional(),
  BASE_SEPOLIA_VALIDATION_ADDRESS: ethereumAddress.optional(),
  BASE_SEPOLIA_START_BLOCK: z.coerce.number().int().min(0).optional(),

  // ... other chains

  // ADD YOUR CHAIN HERE
  OPTIMISM_SEPOLIA_IDENTITY_ADDRESS: ethereumAddress.optional(),
  OPTIMISM_SEPOLIA_REPUTATION_ADDRESS: ethereumAddress.optional(),
  OPTIMISM_SEPOLIA_VALIDATION_ADDRESS: ethereumAddress.optional(),
  OPTIMISM_SEPOLIA_START_BLOCK: z.coerce.number().int().min(0).optional(),
};
```

**Key Points**:
- Use same prefix as RPC schema
- `ethereumAddress` validator ensures valid Ethereum address format
- `START_BLOCK` must be non-negative integer
- All contracts optional (chain only enabled if all contracts provided)

#### 2.3 Update Chain Configuration Check

Locate the `hasAtLeastOneChainConfigured()` function and add your chain:

```typescript
function hasAtLeastOneChainConfigured(env: EnvConfig): boolean {
  const chains = [
    {
      prefix: "BASE_SEPOLIA",
      hasRpc: Boolean(
        env.BASE_SEPOLIA_RPC_ALCHEMY ??
        env.BASE_SEPOLIA_RPC_INFURA ??
        env.BASE_SEPOLIA_RPC_QUIKNODE ??
        env.BASE_SEPOLIA_RPC_ANKR ??
        env.BASE_SEPOLIA_RPC_URL
      ),
    },
    // ... other chains

    // ADD YOUR CHAIN HERE
    {
      prefix: "OPTIMISM_SEPOLIA",
      hasRpc: Boolean(
        env.OPTIMISM_SEPOLIA_RPC_ALCHEMY ??
        env.OPTIMISM_SEPOLIA_RPC_INFURA ??
        env.OPTIMISM_SEPOLIA_RPC_QUIKNODE ??
        env.OPTIMISM_SEPOLIA_RPC_ANKR ??
        env.OPTIMISM_SEPOLIA_RPC_URL
      ),
    },
  ];

  return chains.some((chain) => chain.hasRpc);
}
```

**Purpose**: Ensures at least one chain has RPC configuration before starting.

#### 2.4 Update getConfiguredChains()

Locate the `getConfiguredChains()` function and add your chain:

```typescript
export function getConfiguredChains(env: EnvConfig): string[] {
  const configuredChains: string[] = [];

  // Check Base Sepolia
  if (
    env.BASE_SEPOLIA_IDENTITY_ADDRESS &&
    env.BASE_SEPOLIA_REPUTATION_ADDRESS &&
    env.BASE_SEPOLIA_VALIDATION_ADDRESS &&
    (env.BASE_SEPOLIA_RPC_ALCHEMY ?? env.BASE_SEPOLIA_RPC_INFURA ?? /* ... */)
  ) {
    configuredChains.push("baseSepolia");
  }

  // ... other chains

  // ADD YOUR CHAIN HERE
  if (
    env.OPTIMISM_SEPOLIA_IDENTITY_ADDRESS &&
    env.OPTIMISM_SEPOLIA_REPUTATION_ADDRESS &&
    env.OPTIMISM_SEPOLIA_VALIDATION_ADDRESS &&
    (env.OPTIMISM_SEPOLIA_RPC_ALCHEMY ??
     env.OPTIMISM_SEPOLIA_RPC_INFURA ??
     env.OPTIMISM_SEPOLIA_RPC_QUIKNODE ??
     env.OPTIMISM_SEPOLIA_RPC_ANKR ??
     env.OPTIMISM_SEPOLIA_RPC_URL)
  ) {
    configuredChains.push("optimismSepolia");
  }

  return configuredChains;
}
```

**Purpose**: Returns list of fully configured chains for logging and validation.

---

### Step 3: Update Environment Variables

#### 3.1 Update .env.example Template

**File**: `.env.example`

Add a complete section for your new chain:

```bash
#############################################
# Optimism Sepolia Configuration
#############################################

# RPC Providers (configure at least one)
OPTIMISM_SEPOLIA_RPC_ALCHEMY=https://opt-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_KEY
OPTIMISM_SEPOLIA_RPC_INFURA=https://optimism-sepolia.infura.io/v3/YOUR_INFURA_KEY
OPTIMISM_SEPOLIA_RPC_QUIKNODE=https://YOUR-ENDPOINT.optimism-sepolia.quiknode.pro/YOUR_QUIKNODE_KEY
OPTIMISM_SEPOLIA_RPC_ANKR=https://rpc.ankr.com/optimism_sepolia/YOUR_ANKR_KEY
# OPTIMISM_SEPOLIA_RPC_URL=https://sepolia.optimism.io  # Generic fallback

# ERC-8004 Contract Addresses (checksummed format)
OPTIMISM_SEPOLIA_IDENTITY_ADDRESS=0x8004AA63c570c570eBF15376c0dB199918BFe9Fb
OPTIMISM_SEPOLIA_REPUTATION_ADDRESS=0x8004bd8daB57f14Ed299135749a5CB5c42d341BF
OPTIMISM_SEPOLIA_VALIDATION_ADDRESS=0x8004C269D0A5647E51E121FeB226200ECE932d55

# Start block (block number where contracts were deployed)
OPTIMISM_SEPOLIA_START_BLOCK=8675309
```

**Key Points**:
- Include clear section header with chain name
- Show examples for all 4 RPC providers
- Include commented-out generic fallback option
- Use placeholder addresses (or real ones if available)
- Include descriptive comments

#### 3.2 Configure Your Local .env

**File**: `.env` (your local environment)

Copy the section from `.env.example` and replace with real values:

```bash
# Optimism Sepolia - REAL VALUES
OPTIMISM_SEPOLIA_RPC_ALCHEMY=https://opt-sepolia.g.alchemy.com/v2/abc123def456...
OPTIMISM_SEPOLIA_RPC_INFURA=https://optimism-sepolia.infura.io/v3/xyz789...

OPTIMISM_SEPOLIA_IDENTITY_ADDRESS=0x1234567890abcdef1234567890abcdef12345678
OPTIMISM_SEPOLIA_REPUTATION_ADDRESS=0xabcdef1234567890abcdef1234567890abcdef12
OPTIMISM_SEPOLIA_VALIDATION_ADDRESS=0x7890abcdef1234567890abcdef1234567890abcd

OPTIMISM_SEPOLIA_START_BLOCK=5432100
```

**Best Practices**:
- Configure 2+ RPC providers for failover
- Use checksummed addresses (validate with ethers.js or Etherscan)
- Set START_BLOCK to deployment block (reduces initial sync time)
- Keep .env file secure (never commit to git)

---

### Step 4: Verification and Testing

#### 4.1 Validate Configuration

Run the Ponder development server to validate your configuration:

```bash
cd ponder-indexers
pnpm dev
```

**Expected Output** (successful):

```
[env-validation] Validating environment variables...
[env-validation] ✓ Environment validation passed
[env-validation] Configured chains: baseSepolia, lineaSepolia, optimismSepolia

[health-check] Checking RPC provider health...
[health-check] Optimism Sepolia:
[health-check]   ✓ Alchemy: 45ms (block 5432150)
[health-check]   ✓ Infura: 62ms (block 5432150)
[health-check]   ✗ QuickNode: Not configured
[health-check]   ✗ Ankr: Not configured
[health-check] ✓ 2 healthy providers for Optimism Sepolia

[ponder] Starting Ponder...
[ponder] Indexing networks: Base Sepolia, Linea Sepolia, Optimism Sepolia
[ponder] Syncing Optimism Sepolia from block 5432100...
```

#### 4.2 Common Validation Errors

**Error 1: Environment Validation Failed**

```
[env-validation] ✗ Environment validation failed
[env-validation] Error: OPTIMISM_SEPOLIA_IDENTITY_ADDRESS is not a valid Ethereum address
```

**Solution**: Check address format (must start with `0x`, be 42 characters, checksummed).

**Error 2: No Healthy Providers**

```
[health-check] ✗ No healthy RPC providers for Optimism Sepolia
[health-check] Alchemy: Connection timeout
[health-check] Infura: Invalid API key
```

**Solution**: Verify RPC URLs are correct and API keys are valid.

**Error 3: Contract Not Found**

```
[ponder] Error fetching contract code for 0x123...
[ponder] Contract may not be deployed at this address
```

**Solution**: Verify contract addresses and START_BLOCK. Contract must exist at START_BLOCK.

#### 4.3 Test Event Indexing

**Monitor Logs for First Event**:

```bash
# In terminal running ponder dev
[ponder] Optimism Sepolia: Block 5432150 processed (3 events)
[ponder] Event: AgentRegistered { agentId: 42, owner: 0x... }
[ponder] ✓ Inserted into Event Store
```

**Verify in Database**:

```bash
# Connect to PostgreSQL
psql erc8004_backend

# Check events table
SELECT COUNT(*), chain_id, registry
FROM events
WHERE chain_id = 11155420
GROUP BY chain_id, registry;
```

**Expected Output**:

```
 count | chain_id |  registry
-------+----------+-------------
     5 | 11155420 | identity
    12 | 11155420 | reputation
     3 | 11155420 | validation
```

#### 4.4 End-to-End Trigger Test

**Create a test trigger** for your new chain:

```bash
# Using curl (replace JWT token)
curl -X POST http://localhost:8000/api/v1/triggers \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Optimism Sepolia Test Trigger",
    "chain_id": 11155420,
    "registry": "reputation",
    "enabled": true,
    "conditions": [{
      "condition_type": "score_threshold",
      "field": "score",
      "operator": "<",
      "value": "50"
    }],
    "actions": [{
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "123456789",
        "message_template": "Low score on Optimism Sepolia: {{score}}"
      }
    }]
  }'
```

**Trigger an Event** (interact with smart contract on Optimism Sepolia):

```bash
# Use Foundry cast to submit feedback
cast send $REPUTATION_REGISTRY_ADDRESS \
  "submitFeedback(uint256,uint8,bytes32,bytes32,string,bytes32)" \
  42 45 "$(cast --to-bytes32 'quality')" "$(cast --to-bytes32 '')" \
  "ipfs://..." "$(cast keccak 'test')" \
  --rpc-url $OPTIMISM_SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY
```

**Verify Trigger Execution**:

```bash
# Check action_results table
psql erc8004_backend -c "
SELECT id, trigger_id, status, action_type
FROM action_results
WHERE created_at > NOW() - INTERVAL '5 minutes'
ORDER BY created_at DESC
LIMIT 5;
"
```

**Expected Output**:

```
                  id                  |      trigger_id       | status  | action_type
--------------------------------------+-----------------------+---------+-------------
 ar_abc123...                         | trigger_def456...     | success | telegram
```

---

## Database Considerations

### No Schema Changes Required

The Event Store automatically supports new chains **without database migrations**. Here's why:

**Event Table Structure**:

```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    chain_id INTEGER NOT NULL,  -- ← Supports any chain ID
    block_number BIGINT NOT NULL,
    registry TEXT NOT NULL,     -- identity | reputation | validation
    event_type TEXT NOT NULL,
    -- ... other fields
);

CREATE INDEX idx_events_chain_registry ON events(chain_id, registry);
```

**Key Points**:
- `chain_id` is a standard integer column (no enum)
- Indexes work for all chain IDs automatically
- TimescaleDB hypertable partitions by `created_at`, not `chain_id`

### Checkpoint Management

The `checkpoints` table tracks sync progress per chain:

```sql
CREATE TABLE checkpoints (
    chain_id INTEGER PRIMARY KEY,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Automatic Behavior**:
- Ponder creates checkpoint entry on first sync
- Updated after each block is fully processed
- Used for crash recovery and reorg detection

**Verify Checkpoint**:

```sql
SELECT * FROM checkpoints WHERE chain_id = 11155420;
```

---

## Deployment Guide

### Staging Deployment

#### 1. Update Staging Environment Variables

**Platform**: Render, Railway, Heroku, AWS, etc.

**Add the following environment variables**:

```bash
OPTIMISM_SEPOLIA_RPC_ALCHEMY=https://opt-sepolia.g.alchemy.com/v2/STAGING_KEY
OPTIMISM_SEPOLIA_RPC_INFURA=https://optimism-sepolia.infura.io/v3/STAGING_KEY

OPTIMISM_SEPOLIA_IDENTITY_ADDRESS=0x...
OPTIMISM_SEPOLIA_REPUTATION_ADDRESS=0x...
OPTIMISM_SEPOLIA_VALIDATION_ADDRESS=0x...
OPTIMISM_SEPOLIA_START_BLOCK=5432100
```

**Important**:
- Use separate API keys for staging vs production
- Deploy to staging first to validate configuration
- Monitor logs for 24 hours before production deployment

#### 2. Deploy Code Changes

```bash
git add ponder-indexers/ponder.config.ts
git add ponder-indexers/src/env-validation.ts
git add .env.example
git commit -m "feat(ponder): Add Optimism Sepolia support"
git push origin main  # Or staging branch
```

#### 3. Restart Services

```bash
# Example: Render platform
render services restart ponder-indexers

# Example: Docker Compose
docker-compose restart ponder-indexers
```

#### 4. Monitor Staging Logs

```bash
# Check health check output
curl https://staging.api.8004.dev/api/v1/health | jq .

# Expected response includes:
{
  "ponder": {
    "optimism_sepolia": {
      "healthy": true,
      "latest_block": 5432500,
      "sync_progress": "100%"
    }
  }
}
```

### Production Deployment

#### Pre-Deployment Checklist

- [ ] Staging deployed and stable for 24+ hours
- [ ] All tests passing on staging
- [ ] RPC provider rate limits confirmed sufficient
- [ ] Monitoring dashboards updated (Grafana, Datadog, etc.)
- [ ] Rollback plan documented
- [ ] Team notified of deployment window

#### 1. Production Environment Variables

```bash
# Use production-grade RPC providers
OPTIMISM_SEPOLIA_RPC_ALCHEMY=https://opt-sepolia.g.alchemy.com/v2/PROD_KEY_1
OPTIMISM_SEPOLIA_RPC_INFURA=https://optimism-sepolia.infura.io/v3/PROD_KEY_2
OPTIMISM_SEPOLIA_RPC_QUIKNODE=https://optimism-sepolia.quiknode.pro/PROD_KEY_3

OPTIMISM_SEPOLIA_IDENTITY_ADDRESS=0x...
OPTIMISM_SEPOLIA_REPUTATION_ADDRESS=0x...
OPTIMISM_SEPOLIA_VALIDATION_ADDRESS=0x...
OPTIMISM_SEPOLIA_START_BLOCK=5432100
```

**Best Practices**:
- Use at least 2 RPC providers (preferably 3)
- Store API keys in secret manager (AWS Secrets Manager, HashiCorp Vault)
- Use dedicated production API keys with higher rate limits
- Enable RPC provider monitoring/alerting

#### 2. Deploy with Blue-Green Strategy

```bash
# Option A: Zero-downtime deployment (blue-green)
# 1. Deploy new version to "green" environment
# 2. Verify health checks pass
# 3. Switch traffic to "green"
# 4. Keep "blue" for quick rollback

# Option B: Rolling deployment
# Deploy to one instance at a time, verify, continue
```

#### 3. Post-Deployment Verification

**Immediate Checks** (0-15 minutes):

```bash
# 1. Health check
curl https://api.8004.dev/api/v1/health

# 2. Check Ponder sync status
curl https://api.8004.dev/api/v1/ponder/status

# 3. Verify events being indexed
psql $PROD_DATABASE_URL -c "
SELECT COUNT(*), MAX(created_at)
FROM events
WHERE chain_id = 11155420 AND created_at > NOW() - INTERVAL '5 minutes';
"
```

**Extended Monitoring** (24 hours):

- Event ingestion rate (should match chain's event rate)
- RPC provider latency (p50, p95, p99)
- Error rate (target: <0.1%)
- Trigger execution success rate
- Database query performance

### Rollback Procedure

**If Issues Detected**:

#### Option 1: Disable Chain (Fastest)

```bash
# Remove RPC URLs from environment variables
# Ponder will automatically skip this chain

# Staging/Production dashboard:
# 1. Navigate to environment variables
# 2. Delete or comment out OPTIMISM_SEPOLIA_RPC_* variables
# 3. Restart ponder-indexers service
```

#### Option 2: Revert Code Changes

```bash
# Revert commit
git revert <commit-hash>
git push origin main

# Or rollback to previous version
git checkout <previous-commit>
# Deploy previous version
```

#### Option 3: Full Rollback

```bash
# Roll back entire deployment to previous stable version
# Platform-specific commands:

# Render
render deployments rollback ponder-indexers

# Kubernetes
kubectl rollout undo deployment/ponder-indexers

# Docker Compose
docker-compose down
git checkout <previous-commit>
docker-compose up -d
```

---

## Troubleshooting

### Issue 1: RPC Connection Failures

**Symptoms**:
```
[health-check] ✗ Alchemy: Connection timeout
[health-check] ✗ Infura: HTTP 403 Forbidden
```

**Diagnosis**:

```bash
# Test RPC URL manually
curl -X POST $OPTIMISM_SEPOLIA_RPC_ALCHEMY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Expected response:
{"jsonrpc":"2.0","id":1,"result":"0x52e456"}  # Hex block number
```

**Solutions**:

1. **Invalid API Key**
   - Verify key in provider dashboard
   - Check for trailing spaces in .env file
   - Regenerate API key if necessary

2. **Rate Limit Exceeded**
   - Upgrade provider plan
   - Add additional providers for load balancing
   - Adjust START_BLOCK to reduce initial sync load

3. **Network Issues**
   - Check firewall rules (ensure HTTPS outbound allowed)
   - Verify DNS resolution: `nslookup opt-sepolia.g.alchemy.com`
   - Test from deployment environment (not just local)

### Issue 2: Contract Address Errors

**Symptoms**:
```
[ponder] Error: Contract not found at address 0x123...
[ponder] Possible reasons: wrong network, contract not deployed, incorrect address
```

**Diagnosis**:

```bash
# Verify contract exists on chain
cast code $OPTIMISM_SEPOLIA_IDENTITY_ADDRESS \
  --rpc-url $OPTIMISM_SEPOLIA_RPC_ALCHEMY

# Expected: Non-empty bytecode (0x6080... or similar)
# If "0x": Contract doesn't exist at this address
```

**Solutions**:

1. **Wrong Network**
   - Verify contract was deployed to correct chain (check Etherscan)
   - Common mistake: Using mainnet address on testnet

2. **Incorrect Address**
   - Double-check address from deployment logs
   - Ensure checksummed format (mixed case)
   - Verify in block explorer

3. **START_BLOCK Too Early**
   - Contract might not exist at START_BLOCK
   - Set START_BLOCK to deployment block number
   - Find deployment block in Etherscan transaction

### Issue 3: Event Indexing Stalled

**Symptoms**:
```
[ponder] Optimism Sepolia: Syncing from block 5432100...
[ponder] Progress: 10% (1 hour elapsed)
[ponder] Warning: Sync rate slower than expected
```

**Diagnosis**:

```bash
# Check current block and sync progress
psql erc8004_backend -c "
SELECT chain_id, block_number,
       EXTRACT(EPOCH FROM (NOW() - updated_at)) as seconds_since_update
FROM checkpoints
WHERE chain_id = 11155420;
"
```

**Solutions**:

1. **START_BLOCK Too Low**
   - If chain has millions of blocks, initial sync takes hours
   - Solution: Set START_BLOCK to contract deployment block
   - Example: If deployed at block 5,432,100, use that instead of 0

2. **RPC Provider Throttling**
   - Provider may be rate-limiting requests
   - Solution: Add additional providers for load balancing
   - Check provider dashboard for rate limit status

3. **Large Number of Events**
   - Chain may have high event volume
   - Solution: Increase Ponder's batch size (advanced)
   - Consider using archive node for faster historical sync

### Issue 4: Environment Variable Validation Errors

**Symptoms**:
```
[env-validation] ✗ Environment validation failed
[env-validation] ValidationError: OPTIMISM_SEPOLIA_RPC_ALCHEMY must be HTTPS URL
```

**Common Validation Errors**:

| Error | Cause | Fix |
|-------|-------|-----|
| "must be HTTPS URL" | Using HTTP instead of HTTPS | Change http:// to https:// |
| "not a valid Ethereum address" | Wrong format or length | Use checksummed address (0x + 40 hex chars) |
| "must be positive integer" | START_BLOCK negative or non-integer | Use block number >= 0 |
| "at least one RPC required" | No RPC URLs configured | Add at least one RPC provider URL |

**Debugging Steps**:

```bash
# 1. Check environment variables are loaded
cd ponder-indexers
node -e "console.log(process.env.OPTIMISM_SEPOLIA_RPC_ALCHEMY)"

# 2. Run validation manually
pnpm dev  # Will show detailed validation errors

# 3. Check .env file for common issues
cat .env | grep OPTIMISM_SEPOLIA
# Look for: missing quotes, trailing spaces, special characters
```

### Issue 5: Health Check Failures

**Symptoms**:
```
[health-check] ✗ All RPC providers failed for Optimism Sepolia
[health-check] Cannot start indexing without healthy provider
```

**Diagnosis**:

```typescript
// The health check tests:
// 1. Connection (can reach endpoint?)
// 2. Authentication (valid API key?)
// 3. Block fetching (can get latest block?)
// 4. Latency (response time < 5000ms?)
```

**Solutions**:

1. **Check Provider Status Pages**
   - Alchemy: https://status.alchemy.com
   - Infura: https://status.infura.io
   - QuickNode: https://status.quiknode.com

2. **Verify Network Configuration**
   ```bash
   # Test from your deployment environment
   curl -v https://opt-sepolia.g.alchemy.com/v2/YOUR_KEY

   # Check for SSL/TLS issues, DNS resolution, network policies
   ```

3. **Add Backup Provider**
   - If one provider is down, having multiple prevents downtime
   - Free tier RPC URLs can be used as last resort fallback

---

## Real-World Example: Optimism Sepolia

This section provides a complete, copy-paste ready example for adding **Optimism Sepolia**.

### Complete Code Changes

#### File 1: `ponder-indexers/ponder.config.ts`

**Location: Lines 15-25** (networks object):

```typescript
const networks: Record<string, NetworkConfig> = {
  ethereumSepolia: { chainId: 11155111, name: "Ethereum Sepolia" },
  baseSepolia: { chainId: 84532, name: "Base Sepolia" },
  lineaSepolia: { chainId: 59141, name: "Linea Sepolia" },
  polygonAmoy: { chainId: 80002, name: "Polygon Amoy" },
  ethereumMainnet: { chainId: 1, name: "Ethereum Mainnet" },
  baseMainnet: { chainId: 8453, name: "Base Mainnet" },
  lineaMainnet: { chainId: 59144, name: "Linea Mainnet" },
  optimismSepolia: { chainId: 11155420, name: "Optimism Sepolia" }, // ← ADD THIS
};
```

**Location: Lines 100-110** (contracts object):

```typescript
const contracts: Record<string, Record<string, `0x${string}`>> = {
  // ... existing contracts
  optimismSepolia: { // ← ADD THIS ENTIRE BLOCK
    identity: getContractAddress("OPTIMISM_SEPOLIA_IDENTITY_ADDRESS"),
    reputation: getContractAddress("OPTIMISM_SEPOLIA_REPUTATION_ADDRESS"),
    validation: getContractAddress("OPTIMISM_SEPOLIA_VALIDATION_ADDRESS"),
  },
};
```

**Location: Lines 150-160** (networkEnvPrefixes object):

```typescript
const networkEnvPrefixes: Record<string, string> = {
  ethereumSepolia: "ETHEREUM_SEPOLIA",
  baseSepolia: "BASE_SEPOLIA",
  lineaSepolia: "LINEA_SEPOLIA",
  polygonAmoy: "POLYGON_AMOY",
  ethereumMainnet: "ETHEREUM_MAINNET",
  baseMainnet: "BASE_MAINNET",
  lineaMainnet: "LINEA_MAINNET",
  optimismSepolia: "OPTIMISM_SEPOLIA", // ← ADD THIS
};
```

#### File 2: `ponder-indexers/src/env-validation.ts`

**Location: Lines 20-90** (rpcProviderSchema):

```typescript
const rpcProviderSchema = {
  // ... existing schemas

  // Optimism Sepolia ← ADD THIS ENTIRE BLOCK
  OPTIMISM_SEPOLIA_RPC_ALCHEMY: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_INFURA: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_QUIKNODE: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_ANKR: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_URL: httpsUrl.optional(),
};
```

**Location: Lines 100-180** (contractAddressSchema):

```typescript
const contractAddressSchema = {
  // ... existing schemas

  // Optimism Sepolia ← ADD THIS ENTIRE BLOCK
  OPTIMISM_SEPOLIA_IDENTITY_ADDRESS: ethereumAddress.optional(),
  OPTIMISM_SEPOLIA_REPUTATION_ADDRESS: ethereumAddress.optional(),
  OPTIMISM_SEPOLIA_VALIDATION_ADDRESS: ethereumAddress.optional(),
  OPTIMISM_SEPOLIA_START_BLOCK: z.coerce.number().int().min(0).optional(),
};
```

**Location: Lines 250-300** (hasAtLeastOneChainConfigured function):

```typescript
function hasAtLeastOneChainConfigured(env: EnvConfig): boolean {
  const chains = [
    // ... existing chains
    {
      prefix: "OPTIMISM_SEPOLIA", // ← ADD THIS ENTIRE BLOCK
      hasRpc: Boolean(
        env.OPTIMISM_SEPOLIA_RPC_ALCHEMY ??
        env.OPTIMISM_SEPOLIA_RPC_INFURA ??
        env.OPTIMISM_SEPOLIA_RPC_QUIKNODE ??
        env.OPTIMISM_SEPOLIA_RPC_ANKR ??
        env.OPTIMISM_SEPOLIA_RPC_URL
      ),
    },
  ];
  return chains.some((chain) => chain.hasRpc);
}
```

**Location: Lines 320-400** (getConfiguredChains function):

```typescript
export function getConfiguredChains(env: EnvConfig): string[] {
  const configuredChains: string[] = [];

  // ... existing chain checks

  // Optimism Sepolia ← ADD THIS ENTIRE BLOCK
  if (
    env.OPTIMISM_SEPOLIA_IDENTITY_ADDRESS &&
    env.OPTIMISM_SEPOLIA_REPUTATION_ADDRESS &&
    env.OPTIMISM_SEPOLIA_VALIDATION_ADDRESS &&
    (env.OPTIMISM_SEPOLIA_RPC_ALCHEMY ??
     env.OPTIMISM_SEPOLIA_RPC_INFURA ??
     env.OPTIMISM_SEPOLIA_RPC_QUIKNODE ??
     env.OPTIMISM_SEPOLIA_RPC_ANKR ??
     env.OPTIMISM_SEPOLIA_RPC_URL)
  ) {
    configuredChains.push("optimismSepolia");
  }

  return configuredChains;
}
```

#### File 3: `.env.example`

Add at the end of the file:

```bash
#############################################
# Optimism Sepolia Configuration
#############################################

# RPC Providers (configure at least one)
OPTIMISM_SEPOLIA_RPC_ALCHEMY=https://opt-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_KEY
OPTIMISM_SEPOLIA_RPC_INFURA=https://optimism-sepolia.infura.io/v3/YOUR_INFURA_KEY
OPTIMISM_SEPOLIA_RPC_QUIKNODE=https://YOUR-ENDPOINT.optimism-sepolia.quiknode.pro/YOUR_KEY
OPTIMISM_SEPOLIA_RPC_ANKR=https://rpc.ankr.com/optimism_sepolia/YOUR_ANKR_KEY
# OPTIMISM_SEPOLIA_RPC_URL=https://sepolia.optimism.io  # Public RPC (rate limited)

# ERC-8004 Contract Addresses (checksummed format)
OPTIMISM_SEPOLIA_IDENTITY_ADDRESS=0x8004AA63c570c570eBF15376c0dB199918BFe9Fb
OPTIMISM_SEPOLIA_REPUTATION_ADDRESS=0x8004bd8daB57f14Ed299135749a5CB5c42d341BF
OPTIMISM_SEPOLIA_VALIDATION_ADDRESS=0x8004C269D0A5647E51E121FeB226200ECE932d55

# Start block (block where contracts were deployed)
OPTIMISM_SEPOLIA_START_BLOCK=8675309
```

### Testing Commands

```bash
# 1. Install dependencies
cd ponder-indexers
pnpm install

# 2. Configure .env with real values
cp .env.example .env
# Edit .env and add your Optimism Sepolia RPC URLs and contract addresses

# 3. Validate configuration
pnpm dev

# Expected output:
# [env-validation] ✓ Environment validation passed
# [env-validation] Configured chains: baseSepolia, optimismSepolia
# [health-check] Optimism Sepolia: 2 healthy providers
# [ponder] Starting indexing for Optimism Sepolia...
```

### Expected Log Output

**Successful Startup**:

```
2025-01-30 10:30:00 [env-validation] Validating environment variables...
2025-01-30 10:30:00 [env-validation] ✓ OPTIMISM_SEPOLIA_RPC_ALCHEMY: valid HTTPS URL
2025-01-30 10:30:00 [env-validation] ✓ OPTIMISM_SEPOLIA_IDENTITY_ADDRESS: valid Ethereum address
2025-01-30 10:30:00 [env-validation] ✓ Environment validation passed
2025-01-30 10:30:00 [env-validation] Configured chains: baseSepolia, lineaSepolia, optimismSepolia

2025-01-30 10:30:01 [health-check] Checking RPC provider health for all chains...
2025-01-30 10:30:01 [health-check] Optimism Sepolia:
2025-01-30 10:30:01 [health-check]   ✓ Alchemy: 42ms (block: 8675500)
2025-01-30 10:30:01 [health-check]   ✓ Infura: 58ms (block: 8675500)
2025-01-30 10:30:01 [health-check]   ✗ QuickNode: Not configured
2025-01-30 10:30:01 [health-check]   ✗ Ankr: Not configured
2025-01-30 10:30:01 [health-check] ✓ 2 healthy providers (using fallback + load balancing)

2025-01-30 10:30:02 [ponder] Starting Ponder indexer...
2025-01-30 10:30:02 [ponder] Networks: Base Sepolia (84532), Linea Sepolia (59141), Optimism Sepolia (11155420)
2025-01-30 10:30:02 [ponder] Contracts per network: Identity, Reputation, Validation

2025-01-30 10:30:03 [ponder] Optimism Sepolia: Starting from block 8675309...
2025-01-30 10:30:05 [ponder] Optimism Sepolia: Synced blocks 8675309-8675400 (91 blocks, 12 events)
2025-01-30 10:30:07 [ponder] Optimism Sepolia: Synced blocks 8675401-8675500 (99 blocks, 8 events)
2025-01-30 10:30:08 [ponder] Optimism Sepolia: Caught up to chain head (block 8675500)
2025-01-30 10:30:08 [ponder] ✓ Optimism Sepolia: Real-time indexing active
```

### Verification Queries

```sql
-- Check events are being indexed
SELECT
  chain_id,
  COUNT(*) as event_count,
  MIN(block_number) as first_block,
  MAX(block_number) as latest_block
FROM events
WHERE chain_id = 11155420
GROUP BY chain_id;

-- Expected output:
-- chain_id | event_count | first_block | latest_block
-- ---------+-------------+-------------+--------------
-- 11155420 |          20 |     8675309 |      8675500

-- Check checkpoint status
SELECT * FROM checkpoints WHERE chain_id = 11155420;

-- Expected output:
-- chain_id | block_number | block_hash | updated_at
-- ---------+--------------+------------+---------------------------
-- 11155420 |      8675500 | 0xabc...   | 2025-01-30 10:30:08+00
```

---

## Advanced Topics

### Adding Custom RPC Providers

If your chain uses a provider not in the default list (Alchemy, Infura, QuickNode, Ankr):

#### Step 1: Update env-validation.ts

```typescript
const rpcProviderSchema = {
  // Add custom provider
  OPTIMISM_SEPOLIA_RPC_CUSTOM: httpsUrl.optional(),
  OPTIMISM_SEPOLIA_RPC_CUSTOM_NAME: z.string().optional(), // Optional: provider name for logging
};
```

#### Step 2: Update hasAtLeastOneChainConfigured

```typescript
{
  prefix: "OPTIMISM_SEPOLIA",
  hasRpc: Boolean(
    env.OPTIMISM_SEPOLIA_RPC_ALCHEMY ??
    env.OPTIMISM_SEPOLIA_RPC_INFURA ??
    env.OPTIMISM_SEPOLIA_RPC_QUIKNODE ??
    env.OPTIMISM_SEPOLIA_RPC_ANKR ??
    env.OPTIMISM_SEPOLIA_RPC_CUSTOM ?? // ← ADD THIS
    env.OPTIMISM_SEPOLIA_RPC_URL
  ),
}
```

#### Step 3: Update Health Check (optional)

**File**: `ponder-indexers/src/health-check.ts`

```typescript
export async function getHealthyProviders(envPrefix: string): Promise<string[]> {
  const providers = [
    { name: "Alchemy", url: process.env[`${envPrefix}_RPC_ALCHEMY`] },
    { name: "Infura", url: process.env[`${envPrefix}_RPC_INFURA`] },
    { name: "QuickNode", url: process.env[`${envPrefix}_RPC_QUIKNODE`] },
    { name: "Ankr", url: process.env[`${envPrefix}_RPC_ANKR`] },
    { name: "Custom", url: process.env[`${envPrefix}_RPC_CUSTOM`] }, // ← ADD THIS
    { name: "Generic", url: process.env[`${envPrefix}_RPC_URL`] },
  ];

  // ... health check logic
}
```

### Handling Chains with Different Block Times

Ponder automatically adapts to different block times, but you can optimize:

**Fast Chains** (e.g., Polygon: ~2s blocks):
- No special configuration needed
- Ponder uses real-time WebSocket subscriptions

**Slow Chains** (e.g., Ethereum: ~12s blocks):
- No special configuration needed
- Ponder uses polling with adaptive intervals

**Custom Configuration** (advanced):

```typescript
// ponder.config.ts
export default {
  networks: {
    optimismSepolia: {
      chainId: 11155420,
      transport: http("...", {
        retryCount: 3,
        timeout: 30_000, // 30 seconds
      }),
    },
  },
};
```

### Supporting Chains with Custom Event Formats

Some L2 chains may have slight variations in event encoding:

**Option 1: Use Standard ERC-8004 ABI** (recommended)
- If chain is EVM-compatible, no changes needed
- Standard Solidity event encoding applies

**Option 2: Custom Event Handlers** (if needed)

```typescript
// ponder-indexers/src/IdentityRegistry.ts
import { ponder } from "@/generated";

ponder.on("IdentityRegistry:AgentRegistered", async ({ event, context }) => {
  const { agentId, tokenURI, owner } = event.args;

  // Chain-specific transformation
  let normalizedTokenURI = tokenURI;
  if (context.network.chainId === 11155420) { // Optimism Sepolia
    // Apply chain-specific logic if needed
    normalizedTokenURI = transformOptimismURI(tokenURI);
  }

  await context.db.insert("events", {
    // ... use normalizedTokenURI
  });
});
```

### Multi-Contract Scenarios

If your chain has multiple deployments of ERC-8004 contracts:

**Option 1: Index All Deployments**

```typescript
// ponder.config.ts
const contracts = {
  optimismSepolia: {
    identity: [
      getContractAddress("OPTIMISM_SEPOLIA_IDENTITY_V1_ADDRESS"),
      getContractAddress("OPTIMISM_SEPOLIA_IDENTITY_V2_ADDRESS"),
    ],
    // ... same for reputation and validation
  },
};
```

**Option 2: Separate Network Keys**

```typescript
const networks = {
  optimismSepoliaV1: { chainId: 11155420, name: "Optimism Sepolia V1" },
  optimismSepoliaV2: { chainId: 11155420, name: "Optimism Sepolia V2" },
};

const contracts = {
  optimismSepoliaV1: { /* V1 addresses */ },
  optimismSepoliaV2: { /* V2 addresses */ },
};
```

---

## Reference

### Chain Information Resources

- **Chain List** (official chain registry): https://chainlist.org
- **Ethereum RPC Compatibility**: https://ethereum.org/en/developers/docs/apis/json-rpc/
- **Viem Chain Definitions**: https://github.com/wevm/viem/tree/main/src/chains/definitions

### RPC Provider Documentation

- **Alchemy**:
  - Docs: https://docs.alchemy.com
  - Supported Networks: https://docs.alchemy.com/reference/supported-networks

- **Infura**:
  - Docs: https://docs.infura.io
  - Networks: https://docs.infura.io/networks

- **QuickNode**:
  - Docs: https://www.quicknode.com/docs
  - Chains: https://www.quicknode.com/chains

- **Ankr**:
  - Docs: https://www.ankr.com/docs
  - RPC Service: https://www.ankr.com/rpc/

### Project Files

- **Ponder Config**: `ponder-indexers/ponder.config.ts`
- **Environment Validation**: `ponder-indexers/src/env-validation.ts`
- **Health Check Logic**: `ponder-indexers/src/health-check.ts`
- **Event Handlers**: `ponder-indexers/src/*.ts` (IdentityRegistry, ReputationRegistry, ValidationRegistry)
- **Database Schema**: `database/migrations/`
- **API Documentation**: `rust-backend/crates/api-gateway/API_DOCUMENTATION.md`

### Related Guides

- **[Development Setup](./setup.md)** - Local development environment configuration
- **[Testing Strategy](./TESTING_STRATEGY.md)** - Comprehensive testing approach
- **[Ponder README](../../ponder-indexers/README.md)** - Ponder indexer overview
- **[Contracts Documentation](../../ponder-indexers/CONTRACTS.md)** - Contract addresses and ABIs

### External Resources

- **Ponder Framework**: https://ponder.sh
- **Viem Library**: https://viem.sh
- **Zod Validation**: https://zod.dev
- **ERC-8004 Standard**: https://eips.ethereum.org/EIPS/eip-8004

---

## Appendix

### A. Complete Code Diff

**Complete git diff for adding Optimism Sepolia**:

```diff
diff --git a/ponder-indexers/ponder.config.ts b/ponder-indexers/ponder.config.ts
index abc1234..def5678 100644
--- a/ponder-indexers/ponder.config.ts
+++ b/ponder-indexers/ponder.config.ts
@@ -22,6 +22,7 @@ const networks: Record<string, NetworkConfig> = {
   ethereumMainnet: { chainId: 1, name: "Ethereum Mainnet" },
   baseMainnet: { chainId: 8453, name: "Base Mainnet" },
   lineaMainnet: { chainId: 59144, name: "Linea Mainnet" },
+  optimismSepolia: { chainId: 11155420, name: "Optimism Sepolia" },
 };

@@ -107,6 +108,11 @@ const contracts: Record<string, Record<string, `0x${string}`>> = {
     reputation: getContractAddress("LINEA_MAINNET_REPUTATION_ADDRESS"),
     validation: getContractAddress("LINEA_MAINNET_VALIDATION_ADDRESS"),
   },
+  optimismSepolia: {
+    identity: getContractAddress("OPTIMISM_SEPOLIA_IDENTITY_ADDRESS"),
+    reputation: getContractAddress("OPTIMISM_SEPOLIA_REPUTATION_ADDRESS"),
+    validation: getContractAddress("OPTIMISM_SEPOLIA_VALIDATION_ADDRESS"),
+  },
 };

@@ -157,6 +163,7 @@ const networkEnvPrefixes: Record<string, string> = {
   ethereumMainnet: "ETHEREUM_MAINNET",
   baseMainnet: "BASE_MAINNET",
   lineaMainnet: "LINEA_MAINNET",
+  optimismSepolia: "OPTIMISM_SEPOLIA",
 };

diff --git a/ponder-indexers/src/env-validation.ts b/ponder-indexers/src/env-validation.ts
index 123abcd..456efgh 100644
--- a/ponder-indexers/src/env-validation.ts
+++ b/ponder-indexers/src/env-validation.ts
@@ -85,6 +85,12 @@ const rpcProviderSchema = {
   LINEA_MAINNET_RPC_QUIKNODE: httpsUrl.optional(),
   LINEA_MAINNET_RPC_ANKR: httpsUrl.optional(),
   LINEA_MAINNET_RPC_URL: httpsUrl.optional(),
+
+  OPTIMISM_SEPOLIA_RPC_ALCHEMY: httpsUrl.optional(),
+  OPTIMISM_SEPOLIA_RPC_INFURA: httpsUrl.optional(),
+  OPTIMISM_SEPOLIA_RPC_QUIKNODE: httpsUrl.optional(),
+  OPTIMISM_SEPOLIA_RPC_ANKR: httpsUrl.optional(),
+  OPTIMISM_SEPOLIA_RPC_URL: httpsUrl.optional(),
 };

@@ -175,6 +181,11 @@ const contractAddressSchema = {
   LINEA_MAINNET_REPUTATION_ADDRESS: ethereumAddress.optional(),
   LINEA_MAINNET_VALIDATION_ADDRESS: ethereumAddress.optional(),
   LINEA_MAINNET_START_BLOCK: z.coerce.number().int().min(0).optional(),
+
+  OPTIMISM_SEPOLIA_IDENTITY_ADDRESS: ethereumAddress.optional(),
+  OPTIMISM_SEPOLIA_REPUTATION_ADDRESS: ethereumAddress.optional(),
+  OPTIMISM_SEPOLIA_VALIDATION_ADDRESS: ethereumAddress.optional(),
+  OPTIMISM_SEPOLIA_START_BLOCK: z.coerce.number().int().min(0).optional(),
 };

@@ -295,6 +306,15 @@ function hasAtLeastOneChainConfigured(env: EnvConfig): boolean {
         env.LINEA_MAINNET_RPC_URL
       ),
     },
+    {
+      prefix: "OPTIMISM_SEPOLIA",
+      hasRpc: Boolean(
+        env.OPTIMISM_SEPOLIA_RPC_ALCHEMY ??
+        env.OPTIMISM_SEPOLIA_RPC_INFURA ??
+        env.OPTIMISM_SEPOLIA_RPC_QUIKNODE ??
+        env.OPTIMISM_SEPOLIA_RPC_ANKR ??
+        env.OPTIMISM_SEPOLIA_RPC_URL
+      ),
+    },
   ];

@@ -395,6 +415,19 @@ export function getConfiguredChains(env: EnvConfig): string[] {
     configuredChains.push("lineaMainnet");
   }

+  if (
+    env.OPTIMISM_SEPOLIA_IDENTITY_ADDRESS &&
+    env.OPTIMISM_SEPOLIA_REPUTATION_ADDRESS &&
+    env.OPTIMISM_SEPOLIA_VALIDATION_ADDRESS &&
+    (env.OPTIMISM_SEPOLIA_RPC_ALCHEMY ??
+     env.OPTIMISM_SEPOLIA_RPC_INFURA ??
+     env.OPTIMISM_SEPOLIA_RPC_QUIKNODE ??
+     env.OPTIMISM_SEPOLIA_RPC_ANKR ??
+     env.OPTIMISM_SEPOLIA_RPC_URL)
+  ) {
+    configuredChains.push("optimismSepolia");
+  }
+
   return configuredChains;
 }

diff --git a/.env.example b/.env.example
index aaa1111..bbb2222 100644
--- a/.env.example
+++ b/.env.example
@@ -150,3 +150,20 @@ LINEA_MAINNET_REPUTATION_ADDRESS=0x...
 LINEA_MAINNET_VALIDATION_ADDRESS=0x...
 LINEA_MAINNET_START_BLOCK=0

+#############################################
+# Optimism Sepolia Configuration
+#############################################
+
+# RPC Providers (configure at least one)
+OPTIMISM_SEPOLIA_RPC_ALCHEMY=https://opt-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_KEY
+OPTIMISM_SEPOLIA_RPC_INFURA=https://optimism-sepolia.infura.io/v3/YOUR_INFURA_KEY
+OPTIMISM_SEPOLIA_RPC_QUIKNODE=https://YOUR-ENDPOINT.optimism-sepolia.quiknode.pro/YOUR_KEY
+OPTIMISM_SEPOLIA_RPC_ANKR=https://rpc.ankr.com/optimism_sepolia/YOUR_ANKR_KEY
+# OPTIMISM_SEPOLIA_RPC_URL=https://sepolia.optimism.io
+
+# ERC-8004 Contract Addresses
+OPTIMISM_SEPOLIA_IDENTITY_ADDRESS=0x8004AA63c570c570eBF15376c0dB199918BFe9Fb
+OPTIMISM_SEPOLIA_REPUTATION_ADDRESS=0x8004bd8daB57f14Ed299135749a5CB5c42d341BF
+OPTIMISM_SEPOLIA_VALIDATION_ADDRESS=0x8004C269D0A5647E51E121FeB226200ECE932d55
+
+OPTIMISM_SEPOLIA_START_BLOCK=8675309
```

### B. Environment Variable Checklist

Use this checklist when adding a new chain:

- [ ] **RPC Providers** (at least one required):
  - [ ] `{PREFIX}_RPC_ALCHEMY`
  - [ ] `{PREFIX}_RPC_INFURA`
  - [ ] `{PREFIX}_RPC_QUIKNODE`
  - [ ] `{PREFIX}_RPC_ANKR`
  - [ ] `{PREFIX}_RPC_URL` (generic fallback)

- [ ] **Contract Addresses** (all three required):
  - [ ] `{PREFIX}_IDENTITY_ADDRESS`
  - [ ] `{PREFIX}_REPUTATION_ADDRESS`
  - [ ] `{PREFIX}_VALIDATION_ADDRESS`

- [ ] **Configuration**:
  - [ ] `{PREFIX}_START_BLOCK`

- [ ] **Validation**:
  - [ ] All URLs are HTTPS
  - [ ] All addresses are checksummed format (0x + 40 hex chars)
  - [ ] START_BLOCK is >= 0
  - [ ] No trailing spaces in .env file

### C. Deployment Checklist

**Pre-Deployment**:
- [ ] Code changes committed and pushed
- [ ] Tests passing locally
- [ ] `.env.example` updated with example values
- [ ] Documentation updated (this guide, CONTRACTS.md)

**Staging Deployment**:
- [ ] Environment variables configured in staging
- [ ] Services restarted
- [ ] Health check endpoint returns success
- [ ] First event indexed successfully
- [ ] Monitor for 24 hours

**Production Deployment**:
- [ ] Staging stable for 24+ hours
- [ ] Production environment variables configured
- [ ] Multiple RPC providers configured (minimum 2)
- [ ] Blue-green deployment prepared
- [ ] Monitoring dashboards updated
- [ ] Team notified of deployment

**Post-Deployment**:
- [ ] Health check successful
- [ ] Events being indexed in real-time
- [ ] Trigger execution working
- [ ] Error rate < 0.1%
- [ ] RPC latency acceptable (p95 < 500ms)

### D. Post-Deployment Validation Checklist

Run these commands after deployment:

```bash
# 1. Health check
curl https://api.8004.dev/api/v1/health | jq '.ponder.optimism_sepolia'

# 2. Check event count
psql $DATABASE_URL -c "SELECT COUNT(*) FROM events WHERE chain_id = 11155420;"

# 3. Check latest checkpoint
psql $DATABASE_URL -c "SELECT * FROM checkpoints WHERE chain_id = 11155420;"

# 4. Verify trigger execution
psql $DATABASE_URL -c "
SELECT COUNT(*), status
FROM action_results ar
JOIN triggers t ON ar.trigger_id = t.id
WHERE t.chain_id = 11155420 AND ar.created_at > NOW() - INTERVAL '1 hour'
GROUP BY status;
"

# 5. Check RPC latency (from monitoring dashboard)
# Target: p50 < 100ms, p95 < 500ms, p99 < 1000ms

# 6. Check error rate (from logs)
# Target: < 0.1% of requests
```

---

## Summary

Adding a new blockchain network to the ERC-8004 backend is a **well-defined, repeatable process** that typically takes 1.5-2.5 hours:

✅ **Three files to modify**:
1. `ponder-indexers/ponder.config.ts` - Network and contract definitions
2. `ponder-indexers/src/env-validation.ts` - Zod validation schema
3. `.env.example` - Environment variable template

✅ **No database changes required** - Event Store supports all chains automatically

✅ **Built-in health checks** - System validates RPC providers automatically

✅ **Automatic failover** - Multiple providers ensure high availability

✅ **Type-safe configuration** - Zod validation catches errors at startup

The modular architecture and comprehensive testing infrastructure make chain additions low-risk and highly reliable. Follow this guide, and you'll have a new chain indexed and operational in under 3 hours.

**Questions or issues?** See the [Troubleshooting](#troubleshooting) section or check the project's [operations runbook](../operations/RUNBOOK.md).
