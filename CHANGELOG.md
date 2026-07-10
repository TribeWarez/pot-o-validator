# Changelog

## v0.9.12

### Hex-Lattice Fixes
- Atomic lattice operations: single RwLock for coord_to_hash + hash_to_depth
- Generation counter on every insert (TOCTOU protection for block producer)
- Orphaned depth entries cleaned up on coord overwrite
- Challenge carries generation for staleness detection

### P2P Sync & Decentralization
- Block sync protocol: GET /hexchain/blocks?from_height=N&limit=M
- Periodic sync loop: compare heights with peers, request missing blocks
- Block relay (gossip flooding): forward accepted blocks to all other peers
- Relay dedup: track recently relayed block hashes (bounded set, last 1000)
- Mempool reconciliation: GET /internal/mempool/hashes + GET /internal/tx/:hash
- Periodic mempool hash exchange with peers, missing tx recovery
- Auto-select network mode based on BOOTSTRAP_URLS config

## v0.9.11

### Repository
- Added SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SUPPORT.md, LICENSE
- Added GitHub issue templates (bug report, feature request)
- Added PR template
- Added hexchain-p2p documentation (README, SECURITY, CONTRIBUTING)
- Fixed sub-crate license inconsistencies (MIT → GPL-3.0)
- Updated README.md with comprehensive project documentation

## v0.9.10

### CI/CD
- Fixed release.yml to trigger on `pot-o-validator-v*` tags (GitHub Release)
- Fixed upstream-release.yml: independent repo updates (no skip cascade)
- Fixed upstream-release.yml: check GH_PAT secret
- Removed `cargo build/test` from downstream update jobs (dependency ordering fix)
- Added workflow_dispatch to all workflows for manual triggering
- Fixed crates.io category slug (cryptography::cryptocurrency → cryptography)
- Fixed keyword count (max 5 per crate for crates.io compatibility)

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
