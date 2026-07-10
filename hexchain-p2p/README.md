# hexchain-p2p

[![crates.io](https://img.shields.io/crates/v/hexchain-p2p.svg)](https://crates.io/crates/hexchain-p2p)
[![docs.rs](https://img.shields.io/docsrs/hexchain-p2p)](https://docs.rs/hexchain-p2p)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)

Hexagonal lattice consensus and P2P block validation for the PoT-O network.

## Overview

hexchain-p2p implements a hexagonal lattice-based blockchain consensus mechanism. Each block occupies a coordinate in a hex grid, and consensus is determined by neighbor relationships and proof-of-work validation within the lattice structure.

## Key Components

- **Block Validation** — Timestamp bounds, size limits, Merkle root verification, MML compression checks
- **Hex Consensus** — Challenge generation from lattice neighbors, proof submission, depth tracking
- **Lattice Store** — Thread-safe in-memory lattice with atomic persistence
- **Lattice Geometry** — HCP (hexagonal close-packed) coordinate system with 12-neighbor offset tables
- **Difficulty Adjustment** — Target block time adjustment based on recent block intervals

## Usage

```rust
use hexchain_p2p::{HexConsensus, LatticeStore, ConsensusParams};

let params = ConsensusParams::default();
let lattice = LatticeStore::new("lattice.json".into());
let consensus = HexConsensus::new(params, lattice);
```

## Architecture

The lattice uses an offset coordinate system where each cell has up to 12 neighbors arranged in a hexagonal close-packed pattern. Blocks reference their neighbors' hashes, creating a DAG-like structure that provides natural fork resolution through depth-based scoring.

## Crate

- **crates.io:** [crates.io/crates/hexchain-p2p](https://crates.io/crates/hexchain-p2p)
- **Docs:** [docs.rs/hexchain-p2p](https://docs.rs/hexchain-p2p)

## License

[GPL-3.0](LICENSE) — TribeWarez
