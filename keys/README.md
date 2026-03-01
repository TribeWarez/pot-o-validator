# PoT-O Validator keys

This directory is mounted into the container as `/keys`. The validator uses it for the **relayer keypair** (fee payer for on-chain proof submissions).

## Relayer keypair (on-chain submissions)

- **File**: `relayer.json` (Solana keypair JSON array).
- **Env**: `RELAYER_KEYPAIR_PATH` (default in container: `/keys/relayer.json`).

**Generate a new keypair** (on a host with Solana CLI):

```bash
solana-keygen new -o relayer.json
# Copy relayer.json into this keys/ directory
```

**Public key** (for airdrop / verification):

```bash
solana-keygen pubkey relayer.json
```

If `relayer.json` is missing or invalid, the validator still runs but returns stub transaction signatures (`sim_tx_*`) and logs a warning.

## Optional: miner keypair

Some setups use `miner.json` for CLI miners or other tooling. To use that key as the relayer (one key for both), set in `.env` or docker-compose:

```bash
RELAYER_KEYPAIR_PATH=/keys/miner.json
```

Place your existing miner keypair at `pot-o-validator/keys/miner.json`. The validator will use it to pay fees and sign on-chain proof transactions. See the [testnet gateway README](../../README.md) (section “Validator keys (relayer)”) for the full env reference.
