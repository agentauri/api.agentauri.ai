# Real Event Data from Blockchain

This document contains **real transaction data** extracted from blockchain explorers to validate Ponder event indexing. These events were NOT being indexed due to missing event handlers (now fixed).

## Purpose

- **Validation**: Verify that new event handlers correctly process real blockchain events
- **Testing**: Use this data to create integration tests
- **Debugging**: Reference when troubleshooting indexing issues
- **Documentation**: Show real-world examples of ERC-8004 events

---

## Current Production Contracts (ERC-8004 v1.0)

> **Note**: The system now indexes **ERC-8004 v1.0** contracts deployed January 2026.
> See `.env.local` or Terraform for current configuration.

| Registry | Address | Deployed |
|----------|---------|----------|
| **IdentityRegistry** | `0x8004A818BFB912233c491871b3d84c89A494BD9e` | 2026-01-06 |
| **ReputationRegistry** | `0x8004B663056A597Dffe9eCcC1965A193B7388713` | 2026-01-06 |
| **ValidationRegistry** | Not yet deployed | - |

---

## Historical Test Data (Legacy Contracts - Pre-v1.0)

> **Warning**: The event data below is from **LEGACY contracts** (pre-v1.0, 2025).
> These addresses are NO LONGER indexed. Data preserved for historical reference only.

## Ethereum Sepolia (Chain ID: 11155111)

### Legacy Contract Addresses (Pre-v1.0 - DEPRECATED)
- **IdentityRegistry**: `0x8004a6090Cd10A7288092483047B097295Fb8847` *(LEGACY)*
- **ReputationRegistry**: `0x8004B8FD1A363aa02fDC07635C0c5F94f6Af5B7E` *(LEGACY)*
- **ValidationRegistry**: `0x8004CB39f29c09145F24Ad9dDe2A108C1A2cdfC5` *(LEGACY)*

---

### 1. Registered Event ✅ (Handler Already Exists)

**Transaction**: `0x649c52c62f380f9dd4ca8c0e21e2e5a667db8ccb38794354ff07d2dc7bc795bc`

```json
{
  "blockNumber": 9745219,
  "blockTimestamp": "2025-12-01T10:44:00Z",
  "transactionHash": "0x649c52c62f380f9dd4ca8c0e21e2e5a667db8ccb38794354ff07d2dc7bc795bc",
  "logIndex": 17,
  "contractAddress": "0x8004a6090Cd10A7288092483047B097295Fb8847",
  "event": "Registered",
  "parameters": {
    "agentId": 3234,
    "tokenURI": "data:application/json;base64,eyJuYW1lIjoidGVzdCBhZ2FpbiB0ZXN0aW5nIiwiZGVzY3JpcHRpb24iOiJ0aHNpIGlzIGEgdGV3dCB0aGlzIGlzIGEgdGVzdCB0aGlzIGlzIGEgdGVzdCB0aGlzIGFzZGYiLCJpbWFnZSI6Imh0dHBzOi8vaW1hZ2VzLnBleGVscy5jb20vcGhvdG9zLzQ3MzU5L3NxdWlycmVsLXdpbGRsaWZlLW5hdHVyZS1hbmltYWwtNDczNTkuanBlZz9jcz1zcmdiJmRsPW5hdHVyZS1hbmltYWwtZnVyLTQ3MzU5LmpwZyZmbT1qcGciLCJhdHRyaWJ1dGVzIjpbeyJ0cmFpdF90eXBlIjoiVHJ1c3Q6IFJlcHV0YXRpb24iLCJ2YWx1ZSI6IlN1cHBvcnRlZCJ9LH0=",
    "owner": "0xOwnerAddress..."
  }
}
```

**Status**: ✅ Handler exists - should be indexed correctly

---

### 2. UriUpdated Event ❌→✅ (Handler ADDED - Was Missing!)

**Transaction**: `0x80a86dc075b3394fdfaa949ded4b60e4ff3d626349fe5fcedef70f4759349c48`

```json
{
  "blockNumber": 9738763,
  "blockTimestamp": "2025-11-30T13:11:24Z",
  "transactionHash": "0x80a86dc075b3394fdfaa949ded4b60e4ff3d626349fe5fcedef70f4759349c48",
  "logIndex": 34,
  "contractAddress": "0x8004a6090Cd10A7288092483047B097295Fb8847",
  "event": "UriUpdated",
  "parameters": {
    "agentId": 3229,
    "newUri": "ipfs://bafkreifu6si3crqaejyxspc2gpfnpkagawibumki3p73aagyabniplkxwi",
    "updatedBy": "0x1eE99E92735eE2972ecbBAC7DDe18a522793c8b4"
  }
}
```

**Status**: ✅ Handler added - will now be indexed

**User Impact**: This event updates an agent's metadata URI (IPFS link to updated profile/config)

---

### 3. MetadataSet Event ✅ (Handler Already Exists)

**Transaction**: `0x1136984f3983f56e0c841777a8da82fef9cffac1e8048058fd213ea7de1ee621`

```json
{
  "blockNumber": 9734818,
  "blockTimestamp": "2025-11-30T00:01:36Z",
  "transactionHash": "0x1136984f3983f56e0c841777a8da82fef9cffac1e8048058fd213ea7de1ee621",
  "logIndex": 120,
  "contractAddress": "0x8004a6090Cd10A7288092483047B097295Fb8847",
  "event": "MetadataSet",
  "parameters": {
    "agentId": 12021202,
    "key": "tee_platform",
    "value": "0x6F617369732D726F666C"
  }
}
```

**Status**: ✅ Handler exists - should be indexed correctly

**Notes**: `value` is bytes, contains "oasis-rofl" (TEE platform identifier)

---

### 4. Transfer Event ❌→✅ (Handler ADDED - Was Missing!)

**Transaction**: `0x2cfe825328d08a76384cb3d04f47f66ea8461e9e73fa3b79f339a50723b68db9`

```json
{
  "blockNumber": 9690342,
  "blockTimestamp": "2025-11-23T14:27:00Z",
  "transactionHash": "0x2cfe825328d08a76384cb3d04f47f66ea8461e9e73fa3b79f339a50723b68db9",
  "logIndex": 167,
  "contractAddress": "0x8004a6090Cd10A7288092483047B097295Fb8847",
  "event": "Transfer",
  "parameters": {
    "from": "0x361cC24489DA702BFc57c49357AE5151F65CB464",
    "to": "0x470AEf46CEB329075D92a9874977BBF44Fc9D28c",
    "tokenId": 3116
  }
}
```

**Status**: ✅ Handler added - will now be indexed

**User Impact**: This event tracks agent ownership transfers (NFT transfer = agent control transfer)

---

### 5. NewFeedback Event ✅ (Handler Already Exists)

**Transaction**: `0xff21f81c90b501b871d8a4c5b1a3ce62b38ef06bf1308bcdb33ffd788bf44d8a`

```json
{
  "blockNumber": 9745091,
  "blockTimestamp": "2025-12-01T10:18:24Z",
  "transactionHash": "0xff21f81c90b501b871d8a4c5b1a3ce62b38ef06bf1308bcdb33ffd788bf44d8a",
  "logIndex": 9,
  "contractAddress": "0x8004B8FD1A363aa02fDC07635C0c5F94f6Af5B7E",
  "event": "NewFeedback",
  "parameters": {
    "agentId": 18441844,
    "clientAddress": "0x2EC8A3D26b720c7a2B16f582d883F798bEEA3628",
    "score": 100,
    "tag1": "0x8A51716CF2D6BDF7644D27BDE1D3F6555E8F3BFD84E5D6C15AED8FB41188D9FF",
    "tag2": "0x1BE2ED86E1C9DC1B1294ACFE4F2231BD536FFFFB38450A0BEC60CBDBA8FB21B8",
    "feedbackUri": "ipfs://QmbJRRfnmZNTpBBuQU5PhqLvfzxTP9r6wXaLa17NtnUU7d",
    "feedbackHash": "0x083AB01307E13E4201A8DC3816BCA3FA65C76707F698B8B1D72486B0B4BFDCE0"
  }
}
```

**Status**: ✅ Handler exists - should be indexed correctly

---

### 6. FeedbackRevoked Event ❌→✅ (Handler ADDED - Was Missing!)

**Transaction**: `0x62a7dea24714fddce3df24140fb7632605323cc4be0663eb5c76f6c318636525`

```json
{
  "blockNumber": 9728641,
  "blockTimestamp": "2025-11-29T03:25:12Z",
  "transactionHash": "0x62a7dea24714fddce3df24140fb7632605323cc4be0663eb5c76f6c318636525",
  "logIndex": 25,
  "contractAddress": "0x8004B8FD1A363aa02fDC07635C0c5F94f6Af5B7E",
  "event": "FeedbackRevoked",
  "parameters": {
    "agentId": 3062,
    "clientAddress": "0x60F80B75479fb6f511B16801C5C4F148f4001e49",
    "feedbackIndex": 1
  }
}
```

**Status**: ✅ Handler added - will now be indexed

**User Impact**: Critical for reputation accuracy - clients can revoke fraudulent/erroneous feedback

---

### 7. ResponseAppended Event ❌→✅ (Handler ADDED - Was Missing!)

**Transaction**: `0x40c727d1b2e2b6c9d9d3df05b29486d60b203064706f3c2a5d478b4047924176`

```json
{
  "blockNumber": 9676244,
  "blockTimestamp": "2025-11-21T02:45:24Z",
  "transactionHash": "0x40c727d1b2e2b6c9d9d3df05b29486d60b203064706f3c2a5d478b4047924176",
  "logIndex": 36,
  "contractAddress": "0x8004B8FD1A363aa02fDC07635C0c5F94f6Af5B7E",
  "event": "ResponseAppended",
  "parameters": {
    "agentId": 16751675,
    "clientAddress": "0xAb4cE88Db0277E05CFb5eeb346F6dfb635950eD0",
    "feedbackIndex": 1,
    "responder": "0xEB5c43CD9404eBD82c38e2869656F05ccf54B003",
    "responseUri": "data:application/json;base64,eyJyZXNwb25zZSI6Imdvb2QiLCJ0aW1lc3RhbXAiOjE3NjM3MzYzMDkwMzMsInZlcnNpb24iOiIxLjAifQ==",
    "responseHash": "0xA5C7835F00FEB96C5A88639C4002F038FDDCCF9096AFC81B720867C28A7CC3CD"
  }
}
```

**Status**: ✅ Handler added - will now be indexed

**User Impact**: Agent/3rd party can respond to feedback (dispute, clarify, provide context)

**Decoded responseUri**: `{"response":"good","timestamp":1763736309033,"version":"1.0"}`

---

### 8. ValidationResponse Event ✅ (Handler Already Exists)

**Transaction**: `0x31296b6c50b576cec0a0fbabce93821c2e2a5bb6236a7e261fd74c5a9966c68e`

**Note**: Handler exists, should be indexed correctly. See [API Documentation](../rust-backend/crates/api-gateway/API_DOCUMENTATION.md) for full event structure.

---

### 9. ValidationRequest Event ✅ (Handler Already Exists)

**Transaction**: `0xb4c0a31f8152eab784598599c1bb201b1ed20fb76b42eb8bdcee25733b289d2e`

**Note**: Handler exists, should be indexed correctly.

---

## Base Sepolia (Chain ID: 84532)

### Missing Event Data (Need to Add)

**User provided these transactions**:
- Register: `0x8c8f509ac182686bb02c047031891b82e0aeb352987baf064bff385e59c06d52`
- UriUpdated: `0xa901c91bdf34a61a830c817c7a0fb7388f311969cd3789b47d8e79b1f0f69965`
- MetadataSet: `0x31e6a1ecd027e8e1d66877c47d00db20dbe18827acb37717760a5f8510fe3c1f`
- Transfer: Not present (no tx hash provided)
- NewFeedback: `0x5f3137d7da3262697fa194d262f7a61915fa0962cbf1cad2f99c5c82fdfd65af`
- FeedbackRevoked: Not present
- ResponseAppended: `0x9b3cdf1d284f3d79d0f3ef9da4084a27eda206f97c635c0535342245412a8933`
- ValidationResponse: `0x28a7a75c603fd33eeba783599ef675877682c6ce381b9332f145983949d51102`
- ValidationRequest: `0xa621c965e299937a0bf7cb952f8d234cd793eab8c4188e8c616db4e4039b43e3`

**Explorer**: https://sepolia.basescan.org

---

## Linea Sepolia (Chain ID: 59141)

### Missing Event Data (Need to Add)

**User provided these transactions**:
- Register: `0x7a995969891e2c6b8d480086645fc8df62a12df3d3b66b8c52b0bff2978b2a42`
- UriUpdated: Not present
- MetadataSet: `0x4c6995c35a8a338d0d7e0cbb5e2f8468e88809a13f2433f47940e6f41e585b98`
- Transfer: Not present
- NewFeedback: `0x245d5eb6e4edbff68380ff239cb17e8172e8da58bc2c98c0394c8cc336a9596d`
- FeedbackRevoked: Not present
- ResponseAppended: Not present
- ValidationResponse: `0xd832ac8a7f20e05366768db51cbf426d23507e892ff397c64f97e805f5889d20`
- ValidationRequest: `0x68b1f4be144f2828e0f93157c8c5832a4e9b1e24206025625c7b085d93797162`

**Explorer**: https://sepolia.lineascan.build

---

## Summary of Fixes

### Events Fixed (Handlers Added)

1. ✅ **IdentityRegistry:UriUpdated** - Now indexed on all 3 chains
2. ✅ **IdentityRegistry:Transfer** - Now indexed on all 3 chains
3. ✅ **ReputationRegistry:FeedbackRevoked** - Now indexed on all 3 chains
4. ✅ **ReputationRegistry:ResponseAppended** - Now indexed on all 3 chains

### Expected Impact

**Before Fix**: These 4 events were completely invisible to the backend
**After Fix**: Full event coverage, triggers can now fire for:
- URI updates (agent profile changes)
- Ownership transfers (agent control changes)
- Feedback revocations (reputation corrections)
- Response additions (dispute/context)

### Next Steps

1. **Re-sync Ponder** from START_BLOCK to capture historical events
2. **Test Triggers** with these real events
3. **Verify Database** - check events table for new event types
4. **Monitor Logs** - ensure no errors during indexing

---

**Created**: 2025-12-01
**Purpose**: Validate Ponder event handler fixes
**Related PR**: [Link to PR when created]
