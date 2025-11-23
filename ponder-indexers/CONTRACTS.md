# Contract Addresses Configuration

This document explains how to configure ERC-8004 contract addresses for the Ponder indexers.

## Overview

The Ponder indexers monitor three types of ERC-8004 registries across four testnets:

**Registries:**
- Identity Registry
- Reputation Registry
- Validation Registry

**Networks:**
- Ethereum Sepolia (Chain ID: 11155111)
- Base Sepolia (Chain ID: 84532)
- Linea Sepolia (Chain ID: 59141)
- Polygon Amoy (Chain ID: 80002)

## Configuration File

Contract addresses are configured in `ponder.config.ts`. The file contains a `contracts` object with placeholder addresses.

## How to Add Contract Addresses

### Step 1: Get Deployment Information

For each registry on each network, you need:
1. **Contract address** (e.g., `0x1234567890abcdef...`)
2. **Deployment block number** (e.g., `5000000`)

### Step 2: Update ponder.config.ts

Open `ponder.config.ts` and locate the `contracts` object:

```typescript
const contracts: Record<string, Record<string, `0x${string}`>> = {
  ethereumSepolia: {
    identity: "0x0000000000000000000000000000000000000000",
    reputation: "0x0000000000000000000000000000000000000000",
    validation: "0x0000000000000000000000000000000000000000",
  },
  // ... other networks
};
```

Replace the `0x0000...` addresses with your actual contract addresses:

```typescript
const contracts: Record<string, Record<string, `0x${string}`>> = {
  ethereumSepolia: {
    identity: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    reputation: "0x8F2E097E79B1c51Be9cA9dF1c8B5aC2b7ddEEd20",
    validation: "0x9D4E94dB8EfBa94BdBABFC33B7e45e4E5c5e5e5e",
  },
  // ... other networks
};
```

### Step 3: Update Start Blocks

For faster initial sync, update the `startBlock` for each contract in the `contracts` section below:

Find each contract configuration (e.g., `IdentityRegistryEthereumSepolia`) and update:

```typescript
IdentityRegistryEthereumSepolia: {
  network: "ethereumSepolia",
  abi: IdentityRegistryAbi,
  address: contracts.ethereumSepolia.identity,
  startBlock: 5234567, // Replace with actual deployment block
}
```

## Example: Complete Configuration

Here's an example of a fully configured contract set for Ethereum Sepolia:

```typescript
// Contract addresses
const contracts = {
  ethereumSepolia: {
    identity: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    reputation: "0x8F2E097E79B1c51Be9cA9dF1c8B5aC2b7ddEEd20",
    validation: "0x9D4E94dB8EfBa94BdBABFC33B7e45e4E5c5e5e5e",
  },
  // ... other networks
};

// Contract configurations (in the main config object)
contracts: {
  IdentityRegistryEthereumSepolia: {
    network: "ethereumSepolia",
    abi: IdentityRegistryAbi,
    address: contracts.ethereumSepolia.identity,
    startBlock: 5234567, // Deployment block
  },
  ReputationRegistryEthereumSepolia: {
    network: "ethereumSepolia",
    abi: ReputationRegistryAbi,
    address: contracts.ethereumSepolia.reputation,
    startBlock: 5234890,
  },
  ValidationRegistryEthereumSepolia: {
    network: "ethereumSepolia",
    abi: ValidationRegistryAbi,
    address: contracts.ethereumSepolia.validation,
    startBlock: 5235012,
  },
}
```

## Finding Contract Addresses

### From Deployment Script
If you deployed the contracts yourself:
1. Check the deployment script output
2. Look for transaction receipts
3. Check your deployment artifacts

### From Block Explorer
If contracts are already deployed:
1. Go to the network's block explorer:
   - Ethereum Sepolia: https://sepolia.etherscan.io
   - Base Sepolia: https://sepolia.basescan.org
   - Linea Sepolia: https://sepolia.lineascan.build
   - Polygon Amoy: https://amoy.polygonscan.com
2. Search for the contract address
3. Note the deployment block number (first transaction)

### From ERC-8004 Documentation
Check the official ERC-8004 documentation or GitHub repository for deployed contract addresses.

## Verifying Contract Addresses

Before running the indexer, verify your addresses:

1. **Check contract exists:**
   ```bash
   # Using cast (from foundry)
   cast code <CONTRACT_ADDRESS> --rpc-url <RPC_URL>
   ```

2. **Verify it's the correct contract:**
   ```bash
   # Check if it implements expected functions
   cast call <CONTRACT_ADDRESS> "name()" --rpc-url <RPC_URL>
   ```

3. **Get deployment block:**
   ```bash
   # Find first transaction
   cast age <DEPLOY_TX_HASH> --rpc-url <RPC_URL>
   ```

## Testing Configuration

After updating addresses, test the configuration:

```bash
# Dry run to check configuration
pnpm typecheck

# Start in development mode
pnpm dev
```

Monitor the logs for:
- ✅ Successful connection to RPC endpoints
- ✅ Contract addresses loaded
- ✅ Event indexing starting

## Troubleshooting

### "Contract not found" error
- Verify the address is correct
- Check you're using the right network RPC URL
- Ensure the contract is deployed on that network

### "Invalid ABI" error
- Verify ABIs in `abis/` match deployed contracts
- Check contract is actually an ERC-8004 registry
- Ensure contract version matches ABI version

### Slow initial sync
- Update `startBlock` to deployment block
- Use a faster RPC provider
- Consider using archive node for historical data

### Events not appearing
- Verify events exist on-chain using block explorer
- Check event signatures match ABI
- Ensure contract address is correct
- Verify network chain ID matches configuration

## Multi-Network Deployment

If you're deploying contracts across multiple networks:

1. Deploy to each network separately
2. Record addresses in a deployment log:
   ```
   Ethereum Sepolia:
     Identity: 0x...
     Reputation: 0x...
     Validation: 0x...

   Base Sepolia:
     Identity: 0x...
     Reputation: 0x...
     Validation: 0x...
   ```

3. Update all network configurations at once
4. Test each network independently before running all

## Best Practices

1. **Always use deployment block for startBlock** - Don't start from block 0
2. **Verify addresses on block explorer** - Ensure contracts are verified
3. **Keep addresses in version control** - Commit `ponder.config.ts` changes
4. **Document deployment dates** - Helps with debugging sync issues
5. **Use environment variables for sensitive data** - Keep RPC URLs in `.env`
6. **Test on one network first** - Verify configuration before adding all networks

## Need Help?

- Check the [Ponder documentation](https://ponder.sh)
- Review the [README.md](./README.md) for setup instructions
- Open an issue on GitHub
- Check the ERC-8004 specification
