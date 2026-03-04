# pot-o-mining

[![crates.io](https://img.shields.io/crates/v/pot-o-mining.svg)](https://crates.io/crates/pot-o-mining)
[![docs.rs](https://img.shields.io/docsrs/pot-o-mining)](https://docs.rs/pot-o-mining)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-mining/ci.yml?branch=main)](https://github.com/TribeWarez/pot-o-mining/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Mining coordination and neural-path logic for PoT-O.

- **Crate:** [crates.io/crates/pot-o-mining](https://crates.io/crates/pot-o-mining)
- **Docs:** [docs.rs/pot-o-mining](https://docs.rs/pot-o-mining)
- **Repository:** [github.com/TribeWarez/pot-o-mining](https://github.com/TribeWarez/pot-o-mining)

## Usage

```toml
[dependencies]
pot-o-mining = "0.1"
```

Depends on `pot-o-core` and `ai3-lib`.

## Versioning

Releases follow semantic versioning. To publish:

1. Bump `version` in `Cargo.toml`.
2. Create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
3. CI will publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
