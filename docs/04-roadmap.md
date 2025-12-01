# Auki Network — Web2 → Web3 Roadmap

> **Ship value early. Add proofs gradually. Move on-chain only when stable and measured.**

## Purpose & Scope

* Align the team on **short-term deliverables** and the **long-term end-state**.
* Focus on **incremental steps**, not a **big-bang rewrites**.

References: [Business Logic](./01-business-logic.md), [Web2 Architecture](./02-web2-architecture.md), [Web3 Architecture](./03-web3-architecture.md).

## Principles

* **Feature parity first.** Core UX must be solid before proofs.
* **Centralized validation → On-chain.** Prototype proofs off-chain (Web2 infra) before smart contract migration.
* **One step at a time.** Move Relay, Storage, and Compute independently with clear exit criteria.

## Timeline

*(Off-chain = validated by Web2 infra; On-chain = verified via smart contracts)*

1. **Phase 1 — Finish the MVP.** Web2 feature parity with Reconstruction Node (Compute) and Credits.
2. **Phase 2 — Hagall reborn.** Off-chain Proof of Relay. Relay nodes expose non-public endpoints.
3. **Phase 3 — Every device is a wallet.** On-chain core economy: CreditLedger, Paymaster, Domain policies, ProviderRegistry, SessionManager.
4. **Phase 4 — Trust but verify.** On-chain Proof of Relay.
5. **Phase 5 — Prove what you store.** Off-chain Proof of Storage.
6. **Phase 6 — Store with proof.** On-chain Proof of Storage.
7. **Phase 7 — Show us what you did.** Off-chain Proof of Compute.
8. **Phase 8 — Prove what you did.** On-chain Proof of Compute.

**Note:** Each on-chain phase requires a dedicated security audit. With this plan that’s \~4 audits.

## Next Todo: Web2 Reconstruction Node

> Integrate Compute into the current Web2 architecture before proofs.

* Aligns requirements with recent SDK use cases.
* Register with Discovery; expose health & metadata in Operator Dashboard.
* Define Compute Task API in SDK (spec + inputs commitment; outputs via stream/store).
* Debit Credits via Network Credit Service; add task/job accounting and audit logs.
* No proofs yet — can experiment with TEE, but no validation for now.
* **Acceptance:** End-to-end demo of a reconstruction job in a Domain with credits debited and node rewarded.
