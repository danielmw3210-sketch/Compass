# Architecture Roadmap: Compass 3-Layer Vision

This document maps the evolution of `rust_compass` towards the 3-Layer "Compute & Collateral" Architecture.

---

## 🔹 Layer 1: Proof of History (PoH)
**Status**: ✅ **Operational**
*   **Goal**: High-throughput ordering backbone with cryptographic time.
*   **Done**:
    *   ✅ **Core Loop**: `main.rs` PoH loop.
    *   ✅ **VDF Clock**: `vdf.rs` implementing recursive SHA-256 (120k iter/tick).
    *   ✅ **Persistence**: State restores from disk on restart.
*   **To Do**:
    *   **Leader Schedule**: Rotate signers (currently fixed Admin).
    *   **DB Migration**: Move from JSON to `sled`.

## 🔹 Layer 2: Proof of Mint (Colored Coins)
**Status**: 🚧 **Next Focus**
*   **Goal**: Collateral-backed minting (1 Compass-SOL = Fixed SOL Amount).
*   **Done**:
    *   ✅ **Multi-Asset Wallets**: `wallet.rs` refactored.
    *   ✅ **Asset-Aware Blocks**: Rewards support `asset` field.
    *   ✅ **Genesis Policy**: 100k Foundation Mint.
*   **To Do**:
    *   **Vault Registry**: Define `Vault` (Asset, Ratio, TVL).
    *   **Escrow Oracle**: Admin tool to verify external deposits.
    *   **Mint/Burn Logic**: The bridge mechanics.

## 🔹 Layer 3: Proof of Useful Work (PoUW)
**Status**: 📝 Planned
*   **Goal**: Compute-backed value generation.
*   **To Do**:
    *   **Staked Compute Pools**: Registry for Miners.
    *   **Job Market**: Structs for `JobProposal` and `JobResult`.
    *   **Work Verification**: `verify_work()` function.

---

## Next Steps: Layer 2 Vaults
1.  Create `vault.rs`.
2.  Define `Struct Vault { asset: String, ratio: u64, tvl: u64 }`.
3.  Implement `chain.register_vault()` and `chain.mint_from_vault()`.
