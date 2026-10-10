# Network Modes

## Mode Comparison

| Feature | local_only | vpn_mesh |
|---------|-----------|----------|
| Peer discovery | None | Bootstrap + mDNS |
| Block sync | No | Yes (30s) |
| Mempool relay | No | Yes (60s) |
| Challenge relay | No | Yes |
| Bootstrap registration | No | Yes (60s) |

## local_only

No peer connections. For development and testing.

```toml
peer_network_mode = "local_only"
bootstrap_urls = []
```

## vpn_mesh

Full P2P networking.

```toml
peer_network_mode = "vpn_mesh"
bootstrap_urls = ["http://bootstrap.tribewarez.com/peers"]
enable_mdns = true
```

## Peer Discovery

Bootstrap URLs register every 60 seconds. mDNS advertises on _pot-o-validator._tcp.local.

## Block Sync

Height comparison every 30 seconds, fetch missing blocks, validate, relay to peers.

## Mempool Reconciliation

Every 60 seconds: exchange hashes, fetch missing transactions, validate and add.

## Security

All P2P communication uses ed25519 authenticated transport with signed messages.

## Firewall

| Port | Protocol | Purpose |
|------|----------|---------|
| 8900 | TCP | HTTP API and P2P |
| 5353 | UDP | mDNS |
