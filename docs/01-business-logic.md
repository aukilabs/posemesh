# Posemesh Protocol · Business-Logic Specification

>**Purpose**  
>The primitives, rules and processes that govern how the posemesh protocol operates.

## Background

Posemesh began as a closed Web‑2 solution addressing real‑world alignment problems for enterprise AR. 

The technology proved itself in production, yet still depends on Aukilabs’ centralized infrastructure. 

We are now migrating Posemesh into an open, Web‑3 protocol so that the ecosystem can grow in public.

## Motivation

This document isolates the business logic of Posemesh. 

It is not an architectural or SDK specification; instead it describes the actors, components, and state‑changes that drive the protocol.

## 1. Domain

### Rules

- Created for a fixed $AUKI burn price.
- Uniquely identifies a collection of Spatial Data plus its access rules.
- Ownership is transferable between Participants.
- Requires at least one landmark to define the coordinate system origin.
- **Domain Cluster** - Dynamic swarm of Participants & Providers serving a single Domain.


### Processes

- Transfer history.
- Encrypted M:N data/stream exchange inside the Domain Cluster.
- Encrypted 1:1 messaging inside the Domain Cluster.

## 2. Participants

### Rules

- Any application, node operator, or end‑user possessing a Posemesh identifier is a Participant.
- Participants may own Domains, $AUKI tokens, and Credits.
- A single $AUKI stake qualifies a Participant for one Provider role.

### Processes

- Create a Domain.
- Stake $AUKI to become a Provider(s).
- Grant {Storage|Network|Computing} Provider permissions inside a Domain.
- Grant Participants’ {Storage|Network|Computing} permissions in a Domain.
- Request to operate as a Provider in a Domain Cluster.
- Request to join a Domain Cluster via Network Providers.
- In a Domain Cluster, request to write, read, or compute over Spatial Data.

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

## 4. Credits

### Rules

- Unit of measurement for the computational effort required to interact with Providers (gas).
- Minted when $AUKI is burned, 1 Credit ≈ 1 USD at the burn‑time.
- Locked when a Participant submits a request.
- Debited when that request is fulfilled.
- Non‑transferable; bound to the originating Participant.

### Processes

- Support sponsored Participants (Prefund another Participant).
- Mint history.
- Lock history.
- Debit history.

## 5. Spatial Data

### Rules

Spatial Data is organised into four layers:

1. **Raw layer** – RGB frames, IMU streams, point clouds (raw or intermediary data).
2. **Semantic layer** – calibration landmark/data mapping a Participant into 3‑D space.
3. **Topography layer** – physical occupancy data such as navmeshes.
4. **Rendering layer** – How surfaces look like in a Domain.

### Processes

- Define which Spatial Data types are exchanged in a Domain and its Domain Cluster.

## 6. Spatial Task

### Rules

- A function that consumes and produces Spatial Data.
- Must conform to the Domain’s declared Spatial Data types.
- Has a **requester** (Participant) and a **runner** (Computing Provider).

### Processes

- Participant calibration.
- Domain mapping.
- Domain reconstruction.
- Other spatial algorithms (e.g. SLAM, pathfinding, etc.).

## 7. Providers

## 7.1 Storage Provider

### Rules

- Operates only inside a Domain Cluster.
- Must stake $AUKI; stake is slashed on invalid/missing proofs.

### Processes

- Persist and serve Spatial Data for a specified retention period.
- Generate storage‑integrity and data‑transfer proofs.

## 7.2 Network Provider

### Rules

- May operate within and outside Domain Clusters.
- Must stake $AUKI; stake is slashed on invalid/missing proofs.

### Processes

- Discover Participants/Providers and Domain Clusters.
- Recruit missing Providers in Domain Clusters.
- Provide real‑time networking inside a Domain Cluster.
- Generate proofs of delivered network access.

## 7.3 Computing Provider

### Rules

- Operates only inside Domain Clusters.
- Must stake $AUKI; stake is slashed on invalid/missing proofs.
- May require read/write access to the Domain’s Storage Providers.

### Processes

- Accept Spatial Tasks from Participants.
- Read Spatial Data, perform computation, write results back to Storage or stream to Participants.
- Generate proofs of correct execution.

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

## 9. Portals

### Rules

- A uniquely identified feature point. (AKA landmark).
- Mapped to one or more Domains.
- *Owned and transferable by a Participant*.
- Represents a physical or virtual anchor to the *real-world*.
- Can define a coordinate inside a Domain.

### Processes

- Register Portal under a Domain.
- Reference Portal during calibration (Semantic layer).
- *Transfer ownership independently of its Domain*.
- Allow Participants to scan / reference a Portal.
