# pot-o-validator

[![crates.io](https://img.shields.io/crates/v/pot-o-validator.svg)](https://crates.io/crates/pot-o-validator)
[![docs.rs](https://img.shields.io/docsrs/pot-o-validator)](https://docs.rs/pot-o-validator)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/tribe/pot-o-validator.yml?branch=main)](https://github.com/TribeWarez/tribe/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

PoT-O (Proof of Tensor Optimizations) Validator Service — HTTP API and consensus node for the TribeWarez testnet.

- **Crate:** [crates.io/crates/pot-o-validator](https://crates.io/crates/pot-o-validator)
- **Docs:** [docs.rs/pot-o-validator](https://docs.rs/pot-o-validator)
- **Repository:** [github.com/TribeWarez/tribe](https://github.com/TribeWarez/tribe) (path: `gateway.tribewarez.com/testnet.rpc.gateway.tribewarez.com/pot-o-validator`)

## Crates

- **pot-o-validator** (this crate): binary and library; HTTP API, config, consensus, device registry, extensions bootstrap.
- **pot-o-core**: block/transaction types, errors, constants.
- **ai3-lib**: tensor engine, ESP compat, mining operations.
- **pot-o-mining**: challenge generation, MML/neural path validation, PoT-O consensus.
- **pot-o-extensions**: DeFi, pool strategy, device protocol, chain bridge, peer network, security.

See [docs.tribewarez.com/crates-and-api](https://docs.tribewarez.com/crates-and-api/) and [Implementation Mapping](https://docs.tribewarez.com/public/implementation-map) for details.

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
