# API Reference

## GET /status

Node status and block height.

```json
{
  "status": "ok",
  "block_height": 42,
  "peers": 3,
  "version": "0.9.12",
  "tribechain_enabled": true
}
```

## POST /challenge

Request a mining challenge.

Request body: miner_address (Base58 encoded)

Response: challenge_id, coord, neighbor_hashes, target, generation, timestamp_unix

## POST /submit

Submit a proof for validation.

Request body: challenge_id, proof (nonce, tensor_proof)

Response: accepted (boolean), reward (amount)

## POST /tx

Submit a transaction to the mempool.

Request body: from, to, token, amount, fee, nonce, signature

Response: tx_hash, accepted

## GET /spv/proof

Get SPV Merkle proof for light clients.

Parameters: tx_hash

Response: tx_hash, block_height, merkle_path, confirmed

## GET /peers

List connected peers.

Response: peers array with node_id, address, port, version

## GET /hexchain/blocks

Fetch blocks for sync.

Parameters: from_height (required), limit (default 200)

Response: blocks array with height, block_json, hash

## Internal API

- GET /internal/mempool/hashes — mempool transaction hashes
- GET /internal/tx/:hash — specific transaction by hash
- POST /internal/mint — mint tokens (requires shared-secret auth)

## Rate Limiting

POST /challenge, POST /submit, POST /tx are limited to 60 requests per minute per IP.

## Error Format

```json
{
  "error": "error_code",
  "message": "Human-readable description"
}
```

Common codes: invalid_request (400), unauthorized (401), rate_limited (429), internal_error (500), not_found (404)
