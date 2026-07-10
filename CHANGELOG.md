# Changelog

## v0.9.9

### Security
- Protected `/internal/mint` with shared-secret auth, removed from public router
- Upgraded ed25519-dalek v1 → v2 (CVE-2024-31167)
- Per-IP rate limiting on challenge/submit/tx endpoints
- Docker non-root user (UID 1000), port 8900 no longer host-exposed
- Authenticated DELETE marketplace order endpoint
- Sanitized error messages to prevent information leakage
- Helmet + rate limiting on wallet service
- Rate limiting + address validation on faucet

### Token Economics
- Supply caps on all 7 token types (PTtC, NMTC, AI3 added)
- Integer overflow fixes in cap checks (checked_add)
- Mining rewards use try_issue() to enforce caps

### Blockchain Core
- HexChain difficulty adjustment (30s target block time)
- Block timestamp validation + 1MB size limits
- State root commitment (Merkle tree over balances/nonces)
- PoT-O proof re-verification (tensor re-execution)
- Fixed MML validation for auto-produced blocks

### P2P & Networking
- Authenticated P2P transport (ed25519 node identities)
- SPV Merkle proof API for light clients
- Peer list persistence with atomic writes
- Mempool revalidation on startup

### Infrastructure
- Added `block_height` to /status endpoint
- Windows build compatibility (SIGTERM cfg guard)
- Fixed crates.io category slug
- Miner keypair loaded from file (no deterministic fallback)
