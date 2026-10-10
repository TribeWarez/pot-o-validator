# Troubleshooting

## Port Already in Use

```
Error: Address already in use (os error 98)
```

```bash
lsof -i :8900
kill <pid>
```

Or use a different port:
```bash
PORT=9001 cargo run --release --bin pot-o-validator
```

## Miner Keypair Not Found

```
ERROR Failed to load miner keypair
```

1. Generate: solana-keygen new --outfile keys/miner.json
2. Verify path in config
3. Ensure file contains 64-byte JSON array

## Peers Not Connecting

1. Verify PEER_NETWORK_MODE=vpn_mesh
2. Check BOOTSTRAP_URLS are reachable
3. Ensure firewall allows port 8900
4. Check tribechain_enabled=true on both nodes

## mDNS Not Working

1. Verify enable_mdns=true
2. Check same mdns_service_name on all nodes
3. mDNS does not cross subnets without reflector
4. Use BOOTSTRAP_URLS as fallback

## Block Production Not Starting

1. Check tribechain_enabled=true
2. Verify miner keypair loaded
3. Ensure mempool has transactions
4. Lower difficulty: POT_O_DIFFICULTY=1
5. Check system clock accuracy

## Slow Block Production

1. Lower difficulty
2. Ensure adequate CPU
3. Reduce max_tensor_dim

## Docker: Config Not Loading

```bash
docker run -v $(pwd)/config:/config -p 8900:8900 pot-o-validator
```

## Docker: Permission Denied

```bash
chown -R 1000:1000 ./blockstore
```

## Debug Logging

```bash
RUST_LOG=debug cargo run --release --bin pot-o-validator
```

## Getting Help

1. Check GitHub Issues
2. Open a new issue with the bug report template
3. Include: version, OS, Rust version, config, logs, repro steps
