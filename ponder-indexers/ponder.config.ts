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
// CONTRACT ADDRESSES (from environment variables)
// ============================================================================

// All contract addresses are loaded from environment variables for security
// and flexibility across different environments (dev, staging, production)
const contracts: Record<string, Record<string, `0x${string}`>> = {
  ethereumSepolia: {
    identity: (process.env.ETHEREUM_SEPOLIA_IDENTITY_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    reputation: (process.env.ETHEREUM_SEPOLIA_REPUTATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    validation: (process.env.ETHEREUM_SEPOLIA_VALIDATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
  },
  baseSepolia: {
    identity: (process.env.BASE_SEPOLIA_IDENTITY_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    reputation: (process.env.BASE_SEPOLIA_REPUTATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    validation: (process.env.BASE_SEPOLIA_VALIDATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
  },
  lineaSepolia: {
    identity: (process.env.LINEA_SEPOLIA_IDENTITY_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    reputation: (process.env.LINEA_SEPOLIA_REPUTATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    validation: (process.env.LINEA_SEPOLIA_VALIDATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
  },
  polygonAmoy: {
    identity: (process.env.POLYGON_AMOY_IDENTITY_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    reputation: (process.env.POLYGON_AMOY_REPUTATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
    validation: (process.env.POLYGON_AMOY_VALIDATION_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`,
  },
};

// ============================================================================
// START BLOCKS (from environment variables)
// ============================================================================

// Start block numbers for each network - set to deployment block for faster sync
const startBlocks: Record<string, number> = {
  ethereumSepolia: parseInt(process.env.ETHEREUM_SEPOLIA_START_BLOCK || "0", 10),
  baseSepolia: parseInt(process.env.BASE_SEPOLIA_START_BLOCK || "0", 10),
  lineaSepolia: parseInt(process.env.LINEA_SEPOLIA_START_BLOCK || "0", 10),
  polygonAmoy: parseInt(process.env.POLYGON_AMOY_START_BLOCK || "0", 10),
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
      startBlock: startBlocks.ethereumSepolia,
    },
    IdentityRegistryBaseSepolia: {
      network: "baseSepolia",
      abi: IdentityRegistryAbi,
      address: contracts.baseSepolia.identity,
      startBlock: startBlocks.baseSepolia,
    },
    IdentityRegistryLineaSepolia: {
      network: "lineaSepolia",
      abi: IdentityRegistryAbi,
      address: contracts.lineaSepolia.identity,
      startBlock: startBlocks.lineaSepolia,
    },
    IdentityRegistryPolygonAmoy: {
      network: "polygonAmoy",
      abi: IdentityRegistryAbi,
      address: contracts.polygonAmoy.identity,
      startBlock: startBlocks.polygonAmoy,
    },

    // ========================================================================
    // REPUTATION REGISTRY CONTRACTS
    // ========================================================================
    ReputationRegistryEthereumSepolia: {
      network: "ethereumSepolia",
      abi: ReputationRegistryAbi,
      address: contracts.ethereumSepolia.reputation,
      startBlock: startBlocks.ethereumSepolia,
    },
    ReputationRegistryBaseSepolia: {
      network: "baseSepolia",
      abi: ReputationRegistryAbi,
      address: contracts.baseSepolia.reputation,
      startBlock: startBlocks.baseSepolia,
    },
    ReputationRegistryLineaSepolia: {
      network: "lineaSepolia",
      abi: ReputationRegistryAbi,
      address: contracts.lineaSepolia.reputation,
      startBlock: startBlocks.lineaSepolia,
    },
    ReputationRegistryPolygonAmoy: {
      network: "polygonAmoy",
      abi: ReputationRegistryAbi,
      address: contracts.polygonAmoy.reputation,
      startBlock: startBlocks.polygonAmoy,
    },

    // ========================================================================
    // VALIDATION REGISTRY CONTRACTS
    // ========================================================================
    ValidationRegistryEthereumSepolia: {
      network: "ethereumSepolia",
      abi: ValidationRegistryAbi,
      address: contracts.ethereumSepolia.validation,
      startBlock: startBlocks.ethereumSepolia,
    },
    ValidationRegistryBaseSepolia: {
      network: "baseSepolia",
      abi: ValidationRegistryAbi,
      address: contracts.baseSepolia.validation,
      startBlock: startBlocks.baseSepolia,
    },
    ValidationRegistryLineaSepolia: {
      network: "lineaSepolia",
      abi: ValidationRegistryAbi,
      address: contracts.lineaSepolia.validation,
      startBlock: startBlocks.lineaSepolia,
    },
    ValidationRegistryPolygonAmoy: {
      network: "polygonAmoy",
      abi: ValidationRegistryAbi,
      address: contracts.polygonAmoy.validation,
      startBlock: startBlocks.polygonAmoy,
    },
  },
});
