# pot-o-core

[![crates.io](https://img.shields.io/crates/v/pot-o-core.svg)](https://crates.io/crates/pot-o-core)
[![docs.rs](https://img.shields.io/docsrs/pot-o-core)](https://docs.rs/pot-o-core)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-core/ci.yml?branch=main)](https://github.com/TribeWarez/pot-o-core/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Core types and utilities for PoT-O (Proof of Tensor Optimizations).

- **Crate:** [crates.io/crates/pot-o-core](https://crates.io/crates/pot-o-core)
- **Docs:** [docs.rs/pot-o-core](https://docs.rs/pot-o-core)
- **Repository:** [github.com/TribeWarez/pot-o-core](https://github.com/TribeWarez/pot-o-core)

## Usage

```toml
[dependencies]
pot-o-core = "0.1"
```

## Versioning

Releases follow semantic versioning. To publish a new release:

1. Bump `version` in `Cargo.toml`.
2. Update `CHANGELOG.md` if present.
3. Commit and push, then create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
4. GitHub Actions will run tests, then publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
