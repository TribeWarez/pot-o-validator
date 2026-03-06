# pot-o-validator

PoT-O (Proof of Tensor Optimizations) Validator Service — HTTP API and consensus node for the TribeWarez testnet.

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

## License

MIT — TribeWarez.
