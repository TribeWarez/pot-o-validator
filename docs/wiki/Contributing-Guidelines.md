# Contributing Guidelines

## Code of Conduct

This project follows the Contributor Covenant.

## How to Contribute

1. Check existing issues first
2. Use issue templates for bug reports and feature requests
3. Fork and create a feature branch
4. Run cargo fmt, cargo clippy -- -D warnings, cargo test --release
5. Follow Conventional Commits format
6. Open a pull request

## Development Setup

```bash
git clone https://github.com/TribeWarez/pot-o-validator.git
cd pot-o-validator
cargo build
```

## Testing

```bash
cargo test --release
```

Per-crate: cargo test -p <crate> --release

## CI Pipeline

Every push and PR runs: cargo fmt --check, cargo clippy -- -D warnings, cargo build --release, cargo test --release

## Project Structure

- core/ — Block/transaction types, errors, constants
- ai3-lib/ — Tensor engine, ESP compatibility, mining operations
- mining/ — Challenge generation, PoT-O consensus
- extensions/ — DeFi, staking, peer network, chain bridge
- hexchain-p2p/ — Hexagonal lattice consensus, block validation
- src/ — Validator binary, HTTP API, config
- tests/ — Integration tests
- firmware/ — ESP32/ESP8266 miner firmware

## Commit Format

type(scope): description

Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore

## License

Contributions licensed under GPL-3.0.
