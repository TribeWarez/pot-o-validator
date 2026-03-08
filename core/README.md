# pot-o-core

[![crates.io](https://img.shields.io/crates/v/pot-o-core.svg)](https://crates.io/crates/pot-o-core)
[![docs.rs](https://img.shields.io/docsrs/pot-o-core)](https://docs.rs/pot-o-core)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-core/ci.yml?branch=main)](https://github.com/TribeWarez/pot-o-core/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Core types and utilities for PoT-O (Proof of Tensor Optimizations).

**Current Version**: v0.2.0 | **Parent**: pot-o-validator v0.1.6-alpha

- **Crate:** [crates.io/crates/pot-o-core](https://crates.io/crates/pot-o-core)
- **Docs:** [docs.rs/pot-o-core](https://docs.rs/pot-o-core)
- **Repository:** [github.com/TribeWarez/pot-o-core](https://github.com/TribeWarez/pot-o-core)

## Usage

```toml
[dependencies]
pot-o-core = "0.2"
```

## Core Types & Traits

### Block & Transaction Types

- **Block**: Represents a validated block in the PoT-O chain
- **Transaction**: Individual transaction in a block
- **Proof**: PoT-O mining proof structure
- **Challenge**: Mining challenge with MML and neural path constraints

### Error Handling

```rust
use pot_o_core::{TribeError, TribeResult};

// Custom error types
pub enum TribeError {
    ProofValidationFailed,
    ChallengeExpired,
    PathDistanceTooLarge,
    // ... more variants
}

// Result wrapper
type TribeResult<T> = Result<T, TribeError>;
```

### Tensor Network Types (v0.2.0)

- **EntropyValue**: Fixed-point entropy (1e6 scale)
- **CoherenceFactor**: Device coherence multiplier (0-1e6)
- **TensorState**: Quantum state representation

## Modules

| Module | Purpose | v0.1.x | v0.2.0 |
|--------|---------|--------|--------|
| types | Block/transaction types | ✓ | ✓ |
| error | Error definitions | ✓ | ✓ |
| constants | Network constants | ✓ | ✓ |
| tensor | Tensor network types (new!) | ✗ | ✓ |
| math | Fixed-point arithmetic | ✗ | ✓ |

## Examples

### Working with Proofs

```rust
use pot_o_core::Proof;

let proof = Proof {
    challenge_id: 123,
    mml_score: 1500,
    path_signature: vec![...],
    // ... other fields
};

// Serialize for storage
let bytes = bincode::serialize(&proof)?;
```

### Error Handling

```rust
use pot_o_core::{TribeError, TribeResult};

fn validate_proof(proof: &Proof) -> TribeResult<()> {
    if proof.mml_score < MIN_MML_THRESHOLD {
        return Err(TribeError::ProofValidationFailed);
    }
    Ok(())
}

// Usage
match validate_proof(&proof) {
    Ok(_) => println!("Valid proof"),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

## Dependencies

- **serde**: Serialization/deserialization
- **sha2**: Hash functions
- **hex**: Hex encoding
- **chrono**: Timestamp utilities

## Testing

Run tests with:

```bash
cargo test --lib pot_o_core
```

## Documentation

Full API documentation available at [docs.rs/pot-o-core](https://docs.rs/pot-o-core)

## Versioning

Releases follow semantic versioning. To publish a new release:

1. Bump `version` in `Cargo.toml`.
2. Update `CHANGELOG.md` if present.
3. Commit and push, then create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
4. GitHub Actions will run tests, then publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
