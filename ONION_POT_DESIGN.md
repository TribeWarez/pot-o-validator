## Onion-Pot and Free Internet RPC&RFC – Follow-Up Design Sketch

This document outlines the next-phase architecture for **onion-pot** and the
\"Free Internet RPC&RFC\" layer, to be implemented after the current CI and
modularization work. It does **not** change runtime behavior yet; it serves as
design input for future crates and protocols.

### 1. New workspace member: `onion-pot`

- **Location (planned)**:
  - `gateway.tribewarez.com/testnet.rpc.gateway.tribewarez.com/onion-pot`
- **Crate role**:
  - Implements a **routing and mixing layer** for RPC traffic on top of the
    existing PoT-O validator and Solana/EVM RPC endpoints.
  - Provides onion-style path construction, multi-hop circuits, and
    latency/entropy-aware scheduling for requests and responses.
  - Exposes a Rust API that can be used by:
    - The PoT-O validator service (to serve onion-aware endpoints).
    - Future clients (CLI, browser, firmware gateways) for Free Internet RPC.

Planned module structure (subject to refinement):

- `onion_pot::topology` – node descriptors, link metrics, circuit templates.
- `onion_pot::crypto` – layering, header/mask formats, key derivation hooks
  (using existing `ProofAuthority` and new key material as needed).
- `onion_pot::circuit` – build/extend/tear-down logic for multi-hop paths.
- `onion_pot::rpc` – request/response envelopes, mapping to underlying HTTP/RPC.
- `onion_pot::metrics` – flow statistics, entropy, latency, and reliability.

### 2. Protocol traits (shared with core / extensions)

Introduce small, focused traits that sit between the core PoT-O types and the
onion-pot overlay. These go into either `pot-o-core` or a tiny new crate
`onion-pot-protocol` that both `onion-pot` and `pot-o-validator` depend on.

- `RpcEnvelope` – a serializable wrapper that carries:
  - Logical endpoint (e.g. `status`, `challenge`, `submit`, DeFi method id).
  - Payload bytes (JSON, binary, or future CBOR/MsgPack).
  - Trace identifiers and optional proof-of-transport fields.
- `CircuitId`, `HopId` – typed identifiers for circuits and hops.
- `TransportProof` – minimal interface for proof objects that show a request
  traversed a circuit according to policy (can be backed by PoT-O or simpler
  signatures initially).

These traits are intentionally minimal so they can be backed by:

- Current PoT-O types (`PotOProof`, `ProofPayload`, `Block`, etc.).
- Future quantum-/physics-informed tensor proofs without changing validator
  APIs.

### 3. Consensus and tensor hooks for quantum-/physics-informed algorithms

The new `TensorEngine` trait in `ai3-lib` and the `PotOConsensus` abstraction
in `pot-o-mining` give us clear seams to plug in more advanced algorithms:

- **`ai3-lib` extensions** (future modules):
  - `tensor::quantum_prone` – operators that:
    - Use stochastic or pseudo-quantum-inspired transforms but export the same
      `Tensor` interface.
    - Respect global constraints such as `ESP_MAX_TENSOR_DIM` and firmware
      limits (`platformio.ini`) via shared constants.
  - `operations::physics` – kernels that encode physical priors (e.g.
    conservation constraints, locality, symmetries) but remain numerically
    stable on current hardware.
- **`pot-o-mining` extensions**:
  - A future `QuantumConsensus` or `PathEnrichedConsensus` implementation that
    also implements the `ConsensusEngine`-style trait (if/when we generalize
    PotO further), using:
    - Tensor trajectories instead of single-step outputs.
    - Additional invariants (e.g. energy-like quantities) embedded in the
      proof.

The **key constraint** is that these advanced engines still implement
`TensorEngine` and present compatible `PotOProof`/`ProofPayload` envelopes so
on-chain and RPC validators can be upgraded gradually.

### 4. Integrating onion-pot with the validator

Once the `onion-pot` crate exists:

- Add it as a workspace member and dependency of `pot-o-validator`.
- Extend the `extensions` crate to include:
  - A `FreeInternetRpc` / `OnionRpc` trait that:
    - Accepts `RpcEnvelope`s.
    - Returns wrapped responses and optional `TransportProof`.
  - A default implementation that:
    - Uses a local-only topology at first (single-hop, no real mixing).
    - Later, composes real multi-hop circuits over peer networks.
- In `pot-o-validator`, add an internal module (e.g. `onion_api`) that:
  - Exposes onion-aware endpoints (e.g. `/onion/status`, `/onion/submit`).
  - Delegates to the `OnionRpc` trait, not directly to underlying HTTP or
    consensus logic.

This keeps onion-pot semantics and Free Internet RPC specifics out of the
existing HTTP handler paths while still sharing `AppState`, consensus, and
extension registries.

### 5. CI considerations for onion-pot

When `onion-pot` is added:

- Extend the root CI workflow (`.github/workflows/pot-o-validator.yml`) to:
  - Include `onion-pot` as a workspace member.
  - Run `cargo test --all --all-features`, which will pick up onion-pot unit
    tests automatically.
- Add focused tests in `onion-pot`:
  - Deterministic circuit building (given fixed RNG seeds).
  - Correct layering/unlayering of envelopes.
  - Basic invariants for latency and entropy metrics.

This ensures that the \"Free Internet RPC&RFC\" layer is validated alongside
core PoT-O consensus and AI3 tensor engines as it evolves.

