# pot-o-mining

[![crates.io](https://img.shields.io/crates/v/pot-o-mining.svg)](https://crates.io/crates/pot-o-mining)
[![docs.rs](https://img.shields.io/docsrs/pot-o-mining)](https://docs.rs/pot-o-mining)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-mining/ci.yml?branch=main)](https://github.com/TribeWarez/pot-o-mining/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Mining coordination and neural-path logic for PoT-O.

**Current Version**: v0.1.6-alpha | **Planned**: v0.2.0 with ServiceRegistry

- **Crate:** [crates.io/crates/pot-o-mining](https://crates.io/crates/pot-o-mining)
- **Docs:** [docs.rs/pot-o-mining](https://docs.rs/pot-o-mining)
- **Repository:** [github.com/TribeWarez/pot-o-mining](https://github.com/TribeWarez/pot-o-mining)

## Usage

```toml
[dependencies]
pot-o-mining = "0.2"
```

Depends on `pot-o-core` and `ai3-lib`.

## Core Components

### Challenge Generator
- Generates mining challenges with MML (Minimal Memory Loss) thresholds
- Manages neural path distance constraints
- Tracks challenge expiration and validity windows

### Proof Validator
- Validates MML score meets threshold
- Verifies neural path distances within limits
- Checks proof computation hash

### Consensus Engine
- Manages proof-of-work difficulty adjustment
- Tracks miner performance and reputation
- Coordinates consensus state transitions

## Service Architecture (v0.2.0)

```rust
pub trait ChallengeGenerator {
    fn generate(&self) -> Challenge;
    fn validate_mml(&self, score: u64) -> bool;
}

pub trait ProofValidator {
    fn validate(&self, proof: &Proof) -> Result<(), ValidationError>;
}

pub trait ConsensusEngine {
    fn process_proof(&mut self, proof: &Proof) -> Result<Block, ConsensusError>;
}
```

## Example: Challenge Generation

```rust
use pot_o_mining::ChallengeGenerator;

let generator = StandardChallengeGenerator::new(config)?;
let challenge = generator.generate();
println!("Challenge ID: {}", challenge.id);
println!("MML Threshold: {}", challenge.mml_threshold);
```

## Testing

Run tests:
```bash
cargo test --lib pot_o_mining
```

## Error Handling

Common errors:
- `ChallengeExpired`: Challenge no longer valid
- `MmlThresholdNotMet`: Proof score too low
- `PathDistanceTooLarge`: Neural path exceeds max

All errors implement `std::error::Error` trait.

## Versioning

Releases follow semantic versioning. To publish:

1. Bump `version` in `Cargo.toml`.
2. Create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
3. CI will publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
