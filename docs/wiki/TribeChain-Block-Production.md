# TribeChain Block Production

## Enabling TribeChain

```toml
tribechain_enabled = true
tribechain_miner_keypair_path = "keys/miner.json"
tribechain_genesis_path = "tribechain_genesis.json"
tribechain_blockstore_path = "blockstore.json"
```

## Miner Keypair Setup

Generate a keypair:

```bash
solana-keypair new --outfile keys/miner.json
```

The file must contain a JSON array of exactly 64 integers (32 bytes secret + 32 bytes public).

## Block Production Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| Block interval | 5 seconds | Attempt rate for new blocks |
| Max nonce | 1,000,000 | PoW nonce search space |
| Max txs per block | 1000 | Maximum transactions per block |
| Max pool size | 10000 | Maximum mempool size |
| Min fee | 0 | Minimum transaction fee |

## Monitoring

Check status:

```bash
curl http://localhost:8900/status
```

Key log messages:

| Message | Meaning |
|---------|---------|
| Block producer miner identity loaded | Keypair loaded |
| Auto block produced | New block created |
| Block broadcast to peers | Block sent to peers |
| could not find valid nonce | PoW target not met |
| Failed to load miner keypair | Keypair missing or invalid |

## Troubleshooting

1. Check tribechain_enabled = true
2. Verify miner keypair file exists
3. Ensure mempool has transactions
4. Lower difficulty if needed
5. Check system clock accuracy
