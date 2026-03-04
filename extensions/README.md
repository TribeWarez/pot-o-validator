# pot-o-extensions

[![crates.io](https://img.shields.io/crates/v/pot-o-extensions.svg)](https://crates.io/crates/pot-o-extensions)
[![docs.rs](https://img.shields.io/docsrs/pot-o-extensions)](https://docs.rs/pot-o-extensions)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-extensions/ci.yml?branch=main)](https://github.com/TribeWarez/pot-o-extensions/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

DeFi, staking, and external-chain extensions for the PoT-O validator.

- **Crate:** [crates.io/crates/pot-o-extensions](https://crates.io/crates/pot-o-extensions)
- **Docs:** [docs.rs/pot-o-extensions](https://docs.rs/pot-o-extensions)
- **Repository:** [github.com/TribeWarez/pot-o-extensions](https://github.com/TribeWarez/pot-o-extensions)

## Usage

```toml
[dependencies]
pot-o-extensions = "0.1"
```

Depends on `pot-o-core`, `ai3-lib`, and `pot-o-mining`.

## Versioning

Releases follow semantic versioning. To publish:

1. Bump `version` in `Cargo.toml`.
2. Create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
3. CI will publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
