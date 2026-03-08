# pot-o-validator

[![crates.io](https://img.shields.io/crates/v/pot-o-validator.svg)](https://crates.io/crates/pot-o-validator)
[![crates.io downloads](https://img.shields.io/crates/d/pot-o-validator.svg)](https://crates.io/crates/pot-o-validator)
[![docs.rs](https://img.shields.io/docsrs/pot-o-validator)](https://docs.rs/pot-o-validator)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-validator/ci.yml?branch=main)](https://github.com/TribeWarez/pot-o-validator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

PoT-O (Proof of Tensor Optimizations) Validator Service — HTTP API and consensus node for the TribeWarez testnet.

**Current Version**: v0.2.0 (Dependency Injection + Tensor Network integration) | **Next**: v0.3.0 with quantum-inspired optimizations

- **Crate:** [crates.io/crates/pot-o-validator](https://crates.io/crates/pot-o-validator)
- **Docs:** [docs.rs/pot-o-validator](https://docs.rs/pot-o-validator)
- **Repository:** [github.com/TribeWarez/pot-o-validator](https://github.com/TribeWarez/pot-o-validator)

## Crates

- **pot-o-validator** (this crate): binary and library; HTTP API, config, consensus, device registry, extensions bootstrap.
- **pot-o-core**: block/transaction types, errors, constants.
- **ai3-lib**: tensor engine, ESP compat, mining operations.
- **pot-o-mining**: challenge generation, MML/neural path validation, PoT-O consensus.
- **pot-o-extensions**: DeFi, pool strategy, device protocol, chain bridge, peer network, security.

See [docs.tribewarez.com/crates-and-api](https://docs.tribewarez.com/crates-and-api/) and [Implementation Mapping](https://docs.tribewarez.com/public/implementation-map) for details.

## Architecture

### v0.2.0 (Current)
- **Dependency Injection** service architecture
- **Service traits** for ProofValidator, ChallengeGenerator, ConsensusEngine
- **ServiceRegistry** for flexible component composition
- **Tensor Network Model** (REALMS Part IV) for entropy-based calculations
- **Event system** for state tracking and off-chain monitoring
- **Comprehensive testing** (140+ tests, 50%+ coverage)

### v0.1.6-alpha (Legacy)
- **Monolithic service design** with modular crate organization
- **HTTP API** for proof submission and validation
- **Device registry** for miner management
- **Challenge generation** with MML and neural path validation
- **Consensus engine** with difficulty adjustment

### v0.3.0 (Planned - see MIGRATION_GUIDE.md)
- **Quantum-inspired optimization** algorithms
- **Cross-chain interoperability** enhancements
- **Advanced pool strategies** with dynamic allocation
- **Machine learning integration** for consensus optimization

## Documentation

- **[CHANGELOG.md](CHANGELOG.md)** - Version history and feature timeline
- **[MIGRATION_GUIDE.md](MIGRATION_GUIDE.md)** - v0.1.x → v0.2.0 upgrade path
- **[SECURITY.md](SECURITY.md)** - Security guidelines and vulnerability reporting

## Error Handling

All crates use **custom error types** and **Result<T> pattern**:

```rust
use pot_o_validator::{TribeError, TribeResult};

// Example
fn validate_proof() -> TribeResult<bool> {
    // Returns Err(TribeError::...) on failure
    Ok(true)
}
```

## Service Architecture (v0.2.0 Pattern)

```rust
// TraitService abstractions (planned for v0.2.0)
pub trait ProofValidator { ... }
pub trait ChallengeGenerator { ... }
pub trait ConsensusEngine { ... }

// Multiple implementations
impl ProofValidator for StandardValidator { ... }
impl ProofValidator for TensorAwareValidator { ... }

// ServiceRegistry for composition
let registry = ServiceRegistry::new(config);
let validator = registry.create_validator()?;
```

## Build & run

```bash
cargo build --release
cargo run --release --bin pot-o-validator
```

## Versioning

Releases follow semantic versioning. To publish a new release:

1. Bump `version` in `Cargo.toml`.
2. Update `CHANGELOG.md` if present.
3. Commit and push, then create a tag: `git tag pot-o-validator-v0.1.1 && git push origin pot-o-validator-v0.1.1`.
4. GitHub Actions will run tests, then publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE) — TribeWarez.
