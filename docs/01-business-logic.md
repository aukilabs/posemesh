# Posemesh Protocol - Business-Logic Specification

## Why read this?

If you plan to:

* use the posemesh in an app,
* run a Storage, Network, or Computing node,
* analyze the token-economic incentives, or
* extend the posemesh implementation

---

## Background & Motivation

Posemesh got started as a closed-source web2 network using AR technology to solve real‑world problems. The technology
proved itself in production, yet still depends on Auki's centralized infrastructure.
For more background, see the [Current Architecture](02-current-architecture.md) document.

We are now transitioning Posemesh to be an open-sourced web3 protocol so that the ecosystem can grow in public. This
document isolates the business logic of Posemesh.

It is not an architectural or SDK specification; instead it describes the actors, components, and state‑changes that
drive the protocol.

---

## 1. Domain

**A unique namespace that groups spatial data plus its access rules.**

### Rules

- Minted for a fixed $AUKI burn price.
- Uniquely identified by asymmetric key.
- Ownership is transferable between Participants.
- Requires at least 3 landmarks to define the coordinate system origin.
- A list of Spatial Data types supported in each Spatial Data layer.
- Every Participant & Provider exchanging data in a single Domain is forming a **Domain Cluster**.

### Processes

- History of ownership transfers

---

## 2. Participants

**Any application, node operator, or end‑user possessing a Posemesh identifier.**

### Rules

- Participants may own Domains, $AUKI tokens, and Credits.
- A single $AUKI stake qualifies a Participant for one Provider role.

### Processes

- Create a Domain.
- Stake $AUKI to become a Provider.
- Grant {Storage|Network|Computing} Provider permissions inside a Domain.
- Grant Participants’ {Storage|Network|Computing} permissions in a Domain.
- Request to operate as a Provider in a Domain Cluster.
- Request to join a Domain Cluster via Network Providers.
- In a Domain Cluster, request to write, read, or compute over Spatial Data.

---

## 3. $AUKI Token

### Rules

- Held by Participants.
- Burning $AUKI triggers a deflationary mint into the Reward Pool.
- Staked tokens are non‑transferable.
- Stakes are partially slashed when a Provider fails to provide valid proofs.

### Processes

- Transfer between Participants (with history).
- Burn to create a Domain.
- Burn to mint Credits.
- Stake and, when necessary, slash.

---

## 4. Credits

**Unit of measurement for the computational effort required to interact with Providers (gas).**

### Rules

- Minted when $AUKI is burned, 1 Credit ≈ 1 USD at the time of burn.
- Locked when a Participant submits a request.
- Debited when that request is fulfilled.
- Non‑transferable; bound to the originating Participant.

### Processes

- Support sponsored Participants (prefund another Participant).
- Mint history.
- Lock history.
- Credit transaction history.

---

## 5. Spatial Data

### Rules

Spatial Data is organised into four layers:

1. **Raw layer** – RGB frames, IMU streams, point clouds (raw or intermediary data).
2. **Semantic layer** – calibration landmark/data mapping a Participant into 3D space.
3. **Topography layer** – physical occupancy data such as navmeshes.
4. **Rendering layer** – What surfaces look like in a Domain (textures).

### Processes

- A global list of Spatial Data types is maintained by the Aukilabs.
- Define which Spatial Data types are exchanged in a Domain and its Domain Cluster.

---

## 6. Spatial Task

**A function that consumes and produces Spatial Data (input and output).**

### Rules

- Must conform to the Domain’s declared Spatial Data types.
- Has a **requester** (Participant) and a **runner** (Computing Provider).

### Types

- Participant calibration.
- Domain mapping.
- Domain reconstruction.
- Other spatial algorithms (e.g. SLAM, pathfinding, raycasting, inference, etc.).

---

## 7. Providers

### Dynamic Staking

Providers are Participants that stake a fixed amount of $AUKI, which increase or decrease baed on their performance.

- Reward: each valid proof adds to the stake.
- Partial Slash: each invalid proof subtracts from the stake.

Example:

- Reward per valid proof: +1 % of the original stake
- Slash per invalid proof: –10 % of the original stake

With those numbers, the stake stops growing when:

```
(valid proofs) × 1 %  ≥  (invalid proofs) × 10 %
invalid / valid  ≤  1 % / 10 %  =  0.10   (10 %)
```

That means you can have at most one failed proof for every ten successful ones (10% of submission failed is acceptable)

the general rule would be `r / p` failure ratio, where *r* is the reward and *p* is the penalty,

## 7.1 Storage Provider

### Rules

- Read and write to storage, only inside Domain Clusters.
- Must stake $AUKI; stake is slashed on invalid/missing proofs.

### Processes

- Persist and serve Spatial Data for a specified retention period.
- Generate storage‑integrity and data‑transfer proofs.

## 7.2 Network Provider

### Rules

- Operates both within and outside Domain Clusters.
- Must stake $AUKI; stake is slashed on invalid/missing proofs.

### Processes

- Discovers Participants/Providers and Domain Clusters.
- Recruits Providers into Domain Clusters when they are needed.
- Encrypts M:N data/stream exchange between Participants in a Domain Cluster.
- Encrypts 1:1 messaging between Participant in a Domain Cluster.
- Generates proofs of delivered network access.

## 7.3 Computing Provider

### Rules

- Operates only inside Domain Clusters.
- Must stake $AUKI; stake is slashed on invalid/missing proofs.
- May require read/write access to the Domain’s Storage Providers.

### Processes

- Accept Spatial Tasks from Participants.
- Read Spatial Data, perform computation, write results back to Storage or stream to Participants.
- Generate proofs of correct execution.

---

## 8. Reward Pool

### Rules

- Holds $AUKI earmarked for Provider rewards.
- Receives a share of every $AUKI burn.
- Rewards are sent to the Provider stake.
- Rewards are only claimable when Providers withdraw their stake.

### Processes

- Validate Provider proofs.
- Emit rewards proportional to Credits debited.
- Slash Provider stakes on invalid/missing proofs.

---

## Further Reading

- [Posemesh Current Architecture](./02-current-architecture.md)
- [Posemesh Web3 Architecture](./03-web3-architecture.md)
