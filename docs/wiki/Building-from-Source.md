# Building from Source

## Prerequisites

### Required

- Rust stable toolchain (edition 2021)
- Git
- OpenSSL development libraries

### Optional

- Docker
- PlatformIO

## Clone the Repository

```bash
git clone https://github.com/TribeWarez/pot-o-validator.git
cd pot-o-validator
```

## Build Commands

### Debug Build

```bash
cargo build
```

### Release Build

```bash
cargo build --release
```

## Docker Build

```bash
docker build -t pot-o-validator .
```

## Verifying the Build

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --release
```

## Troubleshooting

### OpenSSL linking errors

```bash
sudo apt-get install pkg-config libssl-dev
```

### Out of memory during build

```bash
CARGO_BUILD_JOBS=2 cargo build --release
```
