import { createConfig } from "@ponder/core";
import { http } from "viem";

import IdentityRegistryAbi from "./abis/IdentityRegistry.json";
import ReputationRegistryAbi from "./abis/ReputationRegistry.json";
import ValidationRegistryAbi from "./abis/ValidationRegistry.json";

// ============================================================================
// NETWORK CONFIGURATION
// ============================================================================

// Type-safe network configuration
interface NetworkConfig {
  chainId: number;
  name: string;
  rpcUrl: string;
}

const networks: Record<string, NetworkConfig> = {
  ethereumSepolia: {
    chainId: 11155111,
    name: "Ethereum Sepolia",
    rpcUrl: process.env.ETHEREUM_SEPOLIA_RPC_URL || "",
  },
  baseSepolia: {
    chainId: 84532,
    name: "Base Sepolia",
    rpcUrl: process.env.BASE_SEPOLIA_RPC_URL || "",
  },
  lineaSepolia: {
    chainId: 59141,
    name: "Linea Sepolia",
    rpcUrl: process.env.LINEA_SEPOLIA_RPC_URL || "",
  },
  polygonAmoy: {
    chainId: 80002,
    name: "Polygon Amoy",
    rpcUrl: process.env.POLYGON_AMOY_RPC_URL || "",
  },
};

// ============================================================================
// CONTRACT ADDRESSES
// ============================================================================

// Placeholder contract addresses - Update these with actual deployed addresses
// Format: contracts[network][registry] = address
const contracts: Record<string, Record<string, `0x${string}`>> = {
  ethereumSepolia: {
    identity: "0x0000000000000000000000000000000000000000",
    reputation: "0x0000000000000000000000000000000000000000",
    validation: "0x0000000000000000000000000000000000000000",
  },
  baseSepolia: {
    identity: "0x0000000000000000000000000000000000000000",
    reputation: "0x0000000000000000000000000000000000000000",
    validation: "0x0000000000000000000000000000000000000000",
  },
  lineaSepolia: {
    identity: "0x0000000000000000000000000000000000000000",
    reputation: "0x0000000000000000000000000000000000000000",
    validation: "0x0000000000000000000000000000000000000000",
  },
  polygonAmoy: {
    identity: "0x0000000000000000000000000000000000000000",
    reputation: "0x0000000000000000000000000000000000000000",
    validation: "0x0000000000000000000000000000000000000000",
  },
};

// ============================================================================
// PONDER CONFIGURATION
// ============================================================================

export default createConfig({
  // Database configuration - uses PostgreSQL from .env
  database: {
    kind: "postgres",
    connectionString: process.env.DATABASE_URL || "",
  },

  // Network configurations
  networks: {
    ethereumSepolia: {
      chainId: networks.ethereumSepolia.chainId,
      transport: http(networks.ethereumSepolia.rpcUrl),
    },
    baseSepolia: {
      chainId: networks.baseSepolia.chainId,
      transport: http(networks.baseSepolia.rpcUrl),
    },
    lineaSepolia: {
      chainId: networks.lineaSepolia.chainId,
      transport: http(networks.lineaSepolia.rpcUrl),
    },
    polygonAmoy: {
      chainId: networks.polygonAmoy.chainId,
      transport: http(networks.polygonAmoy.rpcUrl),
    },
  },

  // Contract configurations
  contracts: {
    // ========================================================================
    // IDENTITY REGISTRY CONTRACTS
    // ========================================================================
    IdentityRegistryEthereumSepolia: {
      network: "ethereumSepolia",
      abi: IdentityRegistryAbi,
      address: contracts.ethereumSepolia.identity,
      startBlock: 0, // Update with actual deployment block
    },
    IdentityRegistryBaseSepolia: {
      network: "baseSepolia",
      abi: IdentityRegistryAbi,
      address: contracts.baseSepolia.identity,
      startBlock: 0,
    },
    IdentityRegistryLineaSepolia: {
      network: "lineaSepolia",
      abi: IdentityRegistryAbi,
      address: contracts.lineaSepolia.identity,
      startBlock: 0,
    },
    IdentityRegistryPolygonAmoy: {
      network: "polygonAmoy",
      abi: IdentityRegistryAbi,
      address: contracts.polygonAmoy.identity,
      startBlock: 0,
    },

    // ========================================================================
    // REPUTATION REGISTRY CONTRACTS
    // ========================================================================
    ReputationRegistryEthereumSepolia: {
      network: "ethereumSepolia",
      abi: ReputationRegistryAbi,
      address: contracts.ethereumSepolia.reputation,
      startBlock: 0,
    },
    ReputationRegistryBaseSepolia: {
      network: "baseSepolia",
      abi: ReputationRegistryAbi,
      address: contracts.baseSepolia.reputation,
      startBlock: 0,
    },
    ReputationRegistryLineaSepolia: {
      network: "lineaSepolia",
      abi: ReputationRegistryAbi,
      address: contracts.lineaSepolia.reputation,
      startBlock: 0,
    },
    ReputationRegistryPolygonAmoy: {
      network: "polygonAmoy",
      abi: ReputationRegistryAbi,
      address: contracts.polygonAmoy.reputation,
      startBlock: 0,
    },

    // ========================================================================
    // VALIDATION REGISTRY CONTRACTS
    // ========================================================================
    ValidationRegistryEthereumSepolia: {
      network: "ethereumSepolia",
      abi: ValidationRegistryAbi,
      address: contracts.ethereumSepolia.validation,
      startBlock: 0,
    },
    ValidationRegistryBaseSepolia: {
      network: "baseSepolia",
      abi: ValidationRegistryAbi,
      address: contracts.baseSepolia.validation,
      startBlock: 0,
    },
    ValidationRegistryLineaSepolia: {
      network: "lineaSepolia",
      abi: ValidationRegistryAbi,
      address: contracts.lineaSepolia.validation,
      startBlock: 0,
    },
    ValidationRegistryPolygonAmoy: {
      network: "polygonAmoy",
      abi: ValidationRegistryAbi,
      address: contracts.polygonAmoy.validation,
      startBlock: 0,
    },
  },
});
