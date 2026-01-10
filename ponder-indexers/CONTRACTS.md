# Contract Addresses Configuration

> **ERC-8004 v1.0** - Last updated: January 2026

This document explains how to configure ERC-8004 contract addresses for the Ponder indexers using environment variables.

## Current Deployments (ERC-8004 v1.0)

| Network | Identity | Reputation | Validation | Start Block |
|---------|----------|------------|------------|-------------|
| **Ethereum Sepolia** | `0x8004A818BFB912233c491871b3d84c89A494BD9e` | `0x8004B663056A597Dffe9eCcC1965A193B7388713` | Not deployed | 9989393 |
| Base Sepolia | Not deployed | Not deployed | Not deployed | - |
| Linea Sepolia | Not deployed | Not deployed | - | - |

## Overview

The Ponder indexers monitor three types of ERC-8004 registries across multiple networks:

**Registries:**
- Identity Registry
- Reputation Registry
- Validation Registry

**Testnets:**
- Ethereum Sepolia (Chain ID: 11155111)
- Base Sepolia (Chain ID: 84532)
- Linea Sepolia (Chain ID: 59141)
- Polygon Amoy (Chain ID: 80002)

**Mainnets:**
- Ethereum Mainnet (Chain ID: 1)
- Base Mainnet (Chain ID: 8453)
- Linea Mainnet (Chain ID: 59144)

**Total:** 21 contract addresses (3 registries × 7 networks)

## Configuration Method

All contract addresses are managed through **environment variables** for security and flexibility. This approach:

- ✅ Keeps sensitive addresses out of code
- ✅ Allows different addresses per environment (dev, staging, production)
- ✅ Follows security best practices
- ✅ Makes deployment configuration easier

## How to Configure Contract Addresses

### Step 1: Get Deployment Information

For each registry on each network, you need:
1. **Contract address** (e.g., `0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb`)
2. **Deployment block number** (e.g., `5000000`)

### Step 2: Update .env File

Open your `.env` file in the project root and update the contract addresses:

```bash
# ============================================================================
# ERC-8004 CONTRACT ADDRESSES - TESTNETS
# ============================================================================

# Ethereum Sepolia Contract Addresses (ERC-8004 v1.0)
ETHEREUM_SEPOLIA_IDENTITY_ADDRESS=0x8004A818BFB912233c491871b3d84c89A494BD9e
ETHEREUM_SEPOLIA_REPUTATION_ADDRESS=0x8004B663056A597Dffe9eCcC1965A193B7388713
ETHEREUM_SEPOLIA_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_SEPOLIA_START_BLOCK=9989393

# Base Sepolia Contract Addresses (NOT YET DEPLOYED)
BASE_SEPOLIA_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
BASE_SEPOLIA_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_SEPOLIA_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_SEPOLIA_START_BLOCK=0

# Linea Sepolia Contract Addresses (NOT YET DEPLOYED)
LINEA_SEPOLIA_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_SEPOLIA_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_SEPOLIA_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_SEPOLIA_START_BLOCK=0

# Polygon Amoy Contract Addresses (NOT YET DEPLOYED)
POLYGON_AMOY_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
POLYGON_AMOY_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
POLYGON_AMOY_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
POLYGON_AMOY_START_BLOCK=0

# ============================================================================
# ERC-8004 CONTRACT ADDRESSES - MAINNETS
# ============================================================================

# Ethereum Mainnet Contract Addresses
ETHEREUM_MAINNET_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_MAINNET_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_MAINNET_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_MAINNET_START_BLOCK=0

# Base Mainnet Contract Addresses
BASE_MAINNET_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
BASE_MAINNET_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_MAINNET_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_MAINNET_START_BLOCK=0

# Linea Mainnet Contract Addresses
LINEA_MAINNET_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_MAINNET_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_MAINNET_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_MAINNET_START_BLOCK=0
```

### Step 3: Verify Configuration

The `ponder.config.ts` file automatically reads these environment variables. You don't need to modify it.

To verify your configuration is loaded correctly:

```bash
# Start Ponder in dev mode
pnpm dev
```

Check the logs for contract loading confirmation.

## Environment Variables Reference

### Contract Addresses

Each network has three registry addresses:

**Testnets:**

| Network | Variable Name Pattern |
|---------|----------------------|
| Ethereum Sepolia | `ETHEREUM_SEPOLIA_{REGISTRY}_ADDRESS` |
| Base Sepolia | `BASE_SEPOLIA_{REGISTRY}_ADDRESS` |
| Linea Sepolia | `LINEA_SEPOLIA_{REGISTRY}_ADDRESS` |
| Polygon Amoy | `POLYGON_AMOY_{REGISTRY}_ADDRESS` |

**Mainnets:**

| Network | Variable Name Pattern |
|---------|----------------------|
| Ethereum Mainnet | `ETHEREUM_MAINNET_{REGISTRY}_ADDRESS` |
| Base Mainnet | `BASE_MAINNET_{REGISTRY}_ADDRESS` |
| Linea Mainnet | `LINEA_MAINNET_{REGISTRY}_ADDRESS` |

Where `{REGISTRY}` is one of:
- `IDENTITY`
- `REPUTATION`
- `VALIDATION`

### Start Blocks

Each network has one start block variable:

**Testnets:**

| Network | Variable Name |
|---------|---------------|
| Ethereum Sepolia | `ETHEREUM_SEPOLIA_START_BLOCK` |
| Base Sepolia | `BASE_SEPOLIA_START_BLOCK` |
| Linea Sepolia | `LINEA_SEPOLIA_START_BLOCK` |
| Polygon Amoy | `POLYGON_AMOY_START_BLOCK` |

**Mainnets:**

| Network | Variable Name |
|---------|---------------|
| Ethereum Mainnet | `ETHEREUM_MAINNET_START_BLOCK` |
| Base Mainnet | `BASE_MAINNET_START_BLOCK` |
| Linea Mainnet | `LINEA_MAINNET_START_BLOCK` |

**Important:** Set the start block to the deployment block number for faster initial sync. If set to `0`, Ponder will index from genesis (very slow).

## Complete Example

Here's a complete example with all addresses configured:

**Testnets:**

```bash
# Ethereum Sepolia (ERC-8004 v1.0 - LIVE)
ETHEREUM_SEPOLIA_IDENTITY_ADDRESS=0x8004A818BFB912233c491871b3d84c89A494BD9e
ETHEREUM_SEPOLIA_REPUTATION_ADDRESS=0x8004B663056A597Dffe9eCcC1965A193B7388713
ETHEREUM_SEPOLIA_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_SEPOLIA_START_BLOCK=9989393

# Base Sepolia (NOT YET DEPLOYED)
BASE_SEPOLIA_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
BASE_SEPOLIA_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_SEPOLIA_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_SEPOLIA_START_BLOCK=0

# Linea Sepolia (NOT YET DEPLOYED)
LINEA_SEPOLIA_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_SEPOLIA_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_SEPOLIA_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_SEPOLIA_START_BLOCK=0

# Polygon Amoy (NOT YET DEPLOYED)
POLYGON_AMOY_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
POLYGON_AMOY_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
POLYGON_AMOY_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
POLYGON_AMOY_START_BLOCK=0
```

**Mainnets:**

```bash
# Ethereum Mainnet
ETHEREUM_MAINNET_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_MAINNET_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_MAINNET_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
ETHEREUM_MAINNET_START_BLOCK=0

# Base Mainnet
BASE_MAINNET_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
BASE_MAINNET_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_MAINNET_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
BASE_MAINNET_START_BLOCK=0

# Linea Mainnet
LINEA_MAINNET_IDENTITY_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_MAINNET_REPUTATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_MAINNET_VALIDATION_ADDRESS=0x0000000000000000000000000000000000000000
LINEA_MAINNET_START_BLOCK=0
```

**Note:** Mainnet contracts are not yet deployed. The addresses above are placeholders (null addresses). Update these when mainnet contracts are deployed.

## Default Behavior

If an address is not set, it defaults to `0x0000000000000000000000000000000000000000` (null address).

If a start block is not set, it defaults to `0` (genesis block).

⚠️ **Warning:** Running with default addresses will not index any actual events. Always update addresses before running in production.

## Troubleshooting

### Ponder shows "No events found"

- Verify your contract addresses are correct
- Check that the contracts are deployed on the specified networks
- Ensure RPC URLs are working (`ETHEREUM_SEPOLIA_RPC_URL`, etc.)
- Verify the start block is not set too high (after actual deployment)

### "Invalid address format" error

- Ensure addresses start with `0x`
- Addresses must be 42 characters long (0x + 40 hex characters)
- Use checksummed addresses when possible

### Sync is very slow

- Set `START_BLOCK` variables to the deployment block number
- Avoid syncing from block 0 unless necessary
- Check RPC provider rate limits

## Security Best Practices

1. **Never commit the .env file** - It's already in `.gitignore`
2. **Use different addresses per environment** - Dev, production, production
3. **Rotate addresses if compromised** - Update `.env` and restart Ponder
4. **Use read-only RPC endpoints** - No write access needed for indexing
5. **Store production addresses securely** - Use secrets management in production

## Additional Resources

- [Ponder Configuration Docs](https://ponder.sh/docs/config)
- [ERC-8004 Specification](https://eips.ethereum.org/EIPS/eip-8004)
- [Viem Address Documentation](https://viem.sh/docs/glossary/types.html#address)
