//! Custom validators for API request validation

use once_cell::sync::Lazy;
use regex::Regex;

/// Regex pattern for Ethereum address validation (0x + 40 hex chars)
#[allow(dead_code)] // Future feature: ETH address validation for Layer 2 wallet authentication
pub static ETH_ADDRESS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^0x[a-fA-F0-9]{40}$").expect("Invalid ETH address regex"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eth_address_valid() {
        assert!(ETH_ADDRESS_REGEX.is_match("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4"));
        assert!(ETH_ADDRESS_REGEX.is_match("0x0000000000000000000000000000000000000000"));
        assert!(ETH_ADDRESS_REGEX.is_match("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
    }

    #[test]
    fn test_eth_address_invalid_no_prefix() {
        assert!(!ETH_ADDRESS_REGEX.is_match("742d35Cc6634C0532925a3b844Bc9e7595f0bEb4"));
    }

    #[test]
    fn test_eth_address_invalid_too_short() {
        assert!(!ETH_ADDRESS_REGEX.is_match("0x742d35Cc6634C0532925a3b844Bc9e75"));
    }

    #[test]
    fn test_eth_address_invalid_too_long() {
        assert!(!ETH_ADDRESS_REGEX.is_match("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb412"));
    }

    #[test]
    fn test_eth_address_invalid_chars() {
        assert!(!ETH_ADDRESS_REGEX.is_match("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEGH"));
    }
}
