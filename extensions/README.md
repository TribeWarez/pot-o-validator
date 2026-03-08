# pot-o-extensions

[![crates.io](https://img.shields.io/crates/v/pot-o-extensions.svg)](https://crates.io/crates/pot-o-extensions)
[![docs.rs](https://img.shields.io/docsrs/pot-o-extensions)](https://docs.rs/pot-o-extensions)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-extensions/ci.yml?branch=main)](https://github.com/TribeWarez/pot-o-extensions/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

DeFi, staking, and external-chain extensions for the PoT-O validator.

**Current Version**: v0.1.6-alpha | **Planned**: v0.2.0 with ServiceRegistry

- **Crate:** [crates.io/crates/pot-o-extensions](https://crates.io/crates/pot-o-extensions)
- **Docs:** [docs.rs/pot-o-extensions](https://docs.rs/pot-o-extensions)
- **Repository:** [github.com/TribeWarez/pot-o-extensions](https://github.com/TribeWarez/pot-o-extensions)

## Usage

```toml
[dependencies]
pot-o-extensions = "0.2"
```

Depends on `pot-o-core`, `ai3-lib`, and `pot-o-mining`.

## Extension Points

### DeFi Pool Integration
- Liquidity pool coordination with mining rewards
- Fee distribution mechanisms
- Cross-program composability (with Solana contracts)

### Staking Module
- Validator staking for consensus participation
- Reward distribution with tensor-aware calculations
- Lock/unlock mechanisms with entropy-based reductions

### Device Protocol
- Device registration and capability tracking
- Coherence factor management (ASIC, GPU, CPU, Mobile)
- Device metrics and health monitoring

### Chain Bridge
- External chain communication
- Cross-chain proof verification
- Interoperability layer for Solana programs

### Peer Network
- Validator peer discovery
- Block propagation and synchronization
- Consensus message routing

### Security Module
- Signature verification
- Permission management
- Rate limiting and DOS protection

## Trait-Based Design (v0.2.0)

```rust
pub trait DeFiPool {
    fn deposit(&mut self, miner: &Pubkey, amount: u64) -> Result<()>;
    fn claim_rewards(&mut self, miner: &Pubkey) -> Result<u64>;
}

pub trait DeviceRegistry {
    fn register(&mut self, device: Device) -> Result<DeviceId>;
    fn get_coherence(&self, device: &DeviceId) -> f64;
}

pub trait StakingProvider {
    fn stake(&mut self, validator: &Pubkey, amount: u64) -> Result<()>;
    fn unstake(&mut self, validator: &Pubkey, amount: u64) -> Result<()>;
}
```

## Example: Device Registration

```rust
use pot_o_extensions::{DeviceRegistry, Device, DeviceType};

let mut registry = DeviceRegistry::new();
let device = Device {
    pubkey: validator_key,
    device_type: DeviceType::GPU,
    // ... other fields
};

registry.register(device)?;
let coherence = registry.get_coherence(&device_id);
println!("Device coherence: {}", coherence);
```

## Testing

Run tests:
```bash
cargo test --lib pot_o_extensions
```

## Documentation

Full API documentation at [docs.rs/pot-o-extensions](https://docs.rs/pot-o-extensions)

## Versioning

Releases follow semantic versioning. To publish:

1. Bump `version` in `Cargo.toml`.
2. Create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
3. CI will publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
