# Running a Validator Node

## Quick Start

### Using Cargo

```bash
cargo run --release --bin pot-o-validator
```

### Using Docker

```bash
docker build -t pot-o-validator .
docker run -p 8900:8900 pot-o-validator
```

## Verifying the Node

```bash
curl http://localhost:8900/status
```

## Configuration

### Via Environment Variables

```bash
PORT=9000 POT_O_DIFFICULTY=4 PEER_NETWORK_MODE=vpn_mesh BOOTSTRAP_URLS="http://peer1.example.com" cargo run --release --bin pot-o-validator
```

### Via Config File

Config search order:
1. /config/default.toml
2. config/default.toml
3. Built-in defaults

## Running as a Service

### systemd

```ini
[Service]
ExecStart=/opt/pot-o-validator/target/release/pot-o-validator
Restart=on-failure
```

## Data Persistence

| File | Default Path |
|------|-------------|
| Device registry | device_registry.json |
| Ledger | /blockstore/ledger.json |
| State | /blockstore/state.json |
| Lattice | /blockstore/lattice.json |
| Mempool | /blockstore/mempool.json |
| Peer store | /blockstore/peers.json |

## Stopping the Node

SIGTERM and SIGINT trigger graceful shutdown with full state persistence.
