# ai3-lib

[![crates.io](https://img.shields.io/crates/v/ai3-lib.svg)](https://crates.io/crates/ai3-lib)
[![docs.rs](https://img.shields.io/docsrs/ai3-lib)](https://docs.rs/ai3-lib)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/ai3-lib/ci.yml?branch=main)](https://github.com/TribeWarez/ai3-lib/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

AI3 support library for PoT-O validator and miner components.

- **Crate:** [crates.io/crates/ai3-lib](https://crates.io/crates/ai3-lib)
- **Docs:** [docs.rs/ai3-lib](https://docs.rs/ai3-lib)
- **Repository:** [github.com/TribeWarez/ai3-lib](https://github.com/TribeWarez/ai3-lib)

## Usage

```toml
[dependencies]
ai3-lib = "0.1"
```

Depends on `pot-o-core`. When used inside the pot-o-validator workspace, the workspace member is used automatically.

## Versioning

Releases follow semantic versioning. To publish:

1. Bump `version` in `Cargo.toml`.
2. Create a tag: `git tag v0.1.1 && git push origin v0.1.1`.
3. CI will publish to crates.io and create a GitHub Release.

## License

[MIT](LICENSE)
