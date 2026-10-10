# Configuration Reference

## Configuration Priority

1. Environment variables
2. Config file
3. Built-in defaults

## All Configuration Fields

### Node Identity

| Field | Type | Default | Env Var |
|-------|------|---------|---------|
| node_id | string | Random UUID | NODE_ID |
| listen_addr | string | "0.0.0.0" | LISTEN_ADDR |
| port | u16 | 8900 | PORT |
| pot_program_id | string | "" | POT_PROGRAM_ID |

### Mining Parameters

| Field | Type | Default | Env Var |
|-------|------|---------|---------|
| difficulty | u64 | 2 | POT_O_DIFFICULTY |
| max_tensor_dim | usize | 64 | — |
| max_mine_iterations | u64 | 10000 | — |

### Network

| Field | Type | Default | Env Var |
|-------|------|---------|---------|
| peer_network_mode | string | "local_only" | PEER_NETWORK_MODE |
| bootstrap_urls | Vec<string> | [] | BOOTSTRAP_URLS |
| enable_mdns | bool | false | ENABLE_MDNS |
| mdns_service_name | string | "pot-o-validator" | MDNS_SERVICE_NAME |
| internal_api_port | u16 | 8900 | INTERNAL_API_PORT |
| peer_timeout_secs | u64 | 30 | PEER_TIMEOUT_SECS |
| challenge_relay_enabled | bool | true | CHALLENGE_RELAY_ENABLED |

### Consensus

| Field | Type | Default | Env Var |
|-------|------|---------|---------|
| maturity_depth | u64 | 10 | MATURITY_DEPTH |
| symmetry_num | u64 | 1 | SYMMETRY_NUM |
| symmetry_den | u64 | 1 | SYMMETRY_DEN |
| base_target | string | "ffff...ffff" | BASE_TARGET |

### TribeChain

| Field | Type | Default | Env Var |
|-------|------|---------|---------|
| tribechain_enabled | bool | false | TRIBECHAIN_ENABLED |
| tribechain_min_fee | u64 | 0 | TRIBECHAIN_MIN_FEE |
| tribechain_max_pool_size | usize | 10000 | TRIBECHAIN_MAX_POOL_SIZE |
| tribechain_max_txs_per_block | usize | 1000 | TRIBECHAIN_MAX_TXS_PER_BLOCK |
| tribechain_genesis_path | string | "tribechain_genesis.json" | TRIBECHAIN_GENESIS_PATH |
| tribechain_miner_address | string | "" | TRIBECHAIN_MINER_ADDRESS |
| tribechain_miner_keypair_path | string | "keys/miner.json" | TRIBECHAIN_MINER_KEYPAIR_PATH |
| tribechain_blockstore_path | string | "blockstore.json" | TRIBECHAIN_BLOCKSTORE_PATH |

## Auto-Detection

If PEER_NETWORK_MODE is not set: BOOTSTRAP_URLS non-empty gives vpn_mesh, empty gives local_only.
