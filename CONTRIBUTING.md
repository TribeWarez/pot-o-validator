# Contributing to PoT-O Validator

Thank you for your interest in contributing! This document provides guidelines and information for contributors.

## Code of Conduct

This project follows our [Code of Conduct](CODE_OF_CONDUCT.md). Please read it before participating.

## How to Contribute

### Reporting Bugs

1. Check existing [issues](https://github.com/TribeWarez/pot-o-validator/issues) first
2. Use the bug report template when creating a new issue
3. Include reproduction steps, expected vs actual behavior, and environment details

### Suggesting Features

1. Check existing issues and discussions
2. Open a new issue with the `enhancement` label
3. Describe the use case and proposed solution

### Submitting Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes following the guidelines below
4. Write or update tests as needed
5. Run the full test suite (see below)
6. Commit with a clear message following [Conventional Commits](https://www.conventionalcommits.org/)
7. Push to your fork and open a Pull Request

## Development Setup

```bash
git clone https://github.com/TribeWarez/pot-o-validator.git
cd pot-o-validator
cargo build
```

## Code Style

- **Formatter:** rustfmt (run `cargo fmt` before committing)
- **Linter:** clippy with warnings as errors (run `cargo clippy -- -D warnings`)
- **No warnings:** All CI checks must pass with zero warnings

## Testing

```bash
cargo test --release
```

All tests must pass before submitting a PR. Add tests for new functionality.

## CI Pipeline

Every push and PR runs:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo build --release`
4. `cargo test --release`

## Project Structure

- `core/` — Block/transaction types, errors, constants
- `ai3-lib/` — Tensor engine, ESP compatibility, mining operations
- `mining/` — Challenge generation, PoT-O consensus
- `extensions/` — DeFi, staking, peer network, chain bridge
- `hexchain-p2p/` — Hexagonal lattice consensus, block validation
- `src/` — Validator binary, HTTP API, config

## License

By contributing, you agree that your contributions will be licensed under the [GPL-3.0 License](LICENSE).
