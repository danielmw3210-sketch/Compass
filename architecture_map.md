# Architecture Roadmap: Compass 3-Layer Vision

This document maps the evolution of `rust_compass` towards the 3-Layer "Compute & Collateral" Architecture.

---

## 🔹 Layer 1: Proof of History (PoH)
**Status**: ✅ Core Loop Implemented | ⚠️ Centralized
*   **Goal**: High-throughput ordering backbone.
*   **Done**: Basic PoH hashing loop, timestamps, admin signing.
*   **To Do**:
    *   Implement Leader Schedule (rotate signers).
    *   Add VDF (Verifiable Delay Function) to prevent timestamp grinding.

## 🔹 Layer 2: Proof of Mint (Colored Coins)
**Status**: ⚠️ In Progress
*   **Goal**: Collateral-backed minting (1 Compass-SOL = Fixed SOL Amount).
*   **Done**:
    *   ✅ **Multi-Asset Wallets**: `wallet.rs` refactored to hold `Map<Asset, Balance>`.
    *   ✅ **Asset-Aware Blocks**: Rewards now specify the `asset` type.
    *   ✅ **Genesis Policy**: Removed infinite inflation; established 100k Foundation Reserve.
*   **To Do**:
    *   **Vault Registry**: Logic to define valid "External Asset" -> "Compass Asset" mappings.
    *   **Escrow Oracle**: Admin helper to verify `deposit_tx` and trigger `mint`.
    *   **Redemption**: Burn logic to release collateral.

## 🔹 Layer 3: Proof of Useful Work (PoUW)
**Status**: 📝 Planned
*   **Goal**: Compute-backed value generation.
*   **To Do**:
    *   **Staked Compute Pools**: Registry for Miners staking Gomining/NFTs.
    *   **Job Market**: Structs for `JobProposal` and `JobResult`.
    *   **Work Verification**: `verify_work()` function in chain logic.

---

## Next Immediate Steps
1.  **Implement Vault Registry**: Define the struct to track Total Value Locked (TVL) for each asset type.
2.  **Build Mock Oracle**: A CLI command to simulate "I received 5 SOL, mint 5000 Compass-SOL".
