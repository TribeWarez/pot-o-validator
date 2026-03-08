# pot-o-validator Test Coverage Analysis
## Comprehensive Test Target Identification

**Date**: 2026-03-08  
**Status**: Thorough codebase analysis complete  
**Current Test Coverage**: 2 smoke tests (~5%)  
**Target Coverage**: 50+ unit + integration tests  

---

## Executive Summary

This analysis identifies all public functions, traits, modules, and error cases across four major crates in the pot-o-validator project. The analysis focuses on creating targeted, priority-ordered test targets for systematic test development.

**Total Test Targets Identified**: 60+ priority tests across 4 crates  
**Critical Priority**: 25 tests (blocking features)  
**High Priority**: 20 tests (core functionality)  
**Medium Priority**: 15+ tests (edge cases & optimization)

---

# PRIORITY 1 (CRITICAL) - Core Foundation Tests

These are the foundational tests that must pass before anything else works.

## 1. pot-o-core Crate (Error Handling & Types)

### File: `core/src/error.rs`

**Module**: Error Type System  
**Public API**:
- `TribeError` enum (10 variants)
- `TribeResult<T>` type alias

**Key Public Functions/Error Variants**:
1. `TribeError::InvalidOperation(String)` - Invalid state/operation errors
2. `TribeError::ProofValidationFailed(String)` - Proof verification failures  
3. `TribeError::TensorError(String)` - Tensor operation failures
4. `TribeError::TensorNetworkFull` - Network at capacity
5. `TribeError::ChainBridgeError(String)` - Cross-chain operation failures
6. `TribeError::NetworkError(String)` - RPC/network errors
7. `TribeError::DeviceError(String)` - Device protocol errors
8. `TribeError::ConfigError(String)` - Configuration errors
9. `TribeError::SerializationError(String)` - Serialization/deserialization errors
10. `TribeError::IoError` - I/O operation errors

**Test Targets** (Priority 1 - 4 tests):

| # | Test Name | Error Case | Expected Behavior | Priority |
|---|-----------|-----------|-------------------|----------|
| 1.1 | `test_tribe_error_creation_all_variants` | Create all 10 error types | All variants constructible, Display formatting works | CRITICAL |
| 1.2 | `test_tribe_error_display_messages` | Format errors for logging | Message includes context string, no panics | CRITICAL |
| 1.3 | `test_tensor_network_full_error_handling` | TensorNetworkFull special case | Error propagates correctly, distinguishable from other errors | CRITICAL |
| 1.4 | `test_tribe_error_from_io_error` | Convert std::io::Error | IoError variant created from io::Error | CRITICAL |

**Error Cases Requiring Tests**:
- Empty context strings
- Very long error messages (>1000 chars)
- Special characters in error text
- Unicode/emoji in error messages
- Serialization/deserialization roundtrip
- Error cloning (derive Clone)
- Error comparison (derive Debug)

---

### File: `core/src/types/tensor_network.rs`

**Module**: Tensor Network Model (REALMS Part IV)  
**Public Structs**:
1. `TensorNetworkVertex` - Represents miners/pools in the network
2. `EntanglementEdge` - Links between vertices
3. `MinimalCut` - Cut entropy calculation
4. `TensorNetworkState` - Global network state

**Key Public Methods**:

#### TensorNetworkVertex
```rust
pub fn new(pubkey, label, dimension, created_at) -> Self
pub fn dimension: u32  // Constrained to [2, 16]
pub fn entanglement_index: u32
```

#### EntanglementEdge
```rust
pub fn new(id, source, target, bond_dimension, coupling_strength, created_at) -> Self
pub fn bond_dimension: u32  // Constrained to [2, 16]
pub fn coupling_strength: u64  // Constrained to [0, 1e6]
```

#### MinimalCut
```rust
pub fn new(edges) -> Self
pub fn cut_size: u32
pub fn total_bond_dimension: u64
```

#### TensorNetworkState (CRITICAL)
```rust
pub fn new() -> Self
pub fn add_vertex(&mut self, vertex) -> TribeResult<()>  // Capacity: 256 max
pub fn add_edge(&mut self, edge) -> TribeResult<()>  // Capacity: 2048 max
pub fn get_vertex(&self, pubkey) -> Option<&TensorNetworkVertex>
pub fn incident_edges(&self, pubkey) -> Vec<&EntanglementEdge>
pub fn vertex_count: u32
pub fn edge_count: u32
```

**Test Targets** (Priority 1 - 10 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 2.1 | `test_tensor_network_vertex_creation_valid` | Create vertex with valid params | Vertex created, dimension clamped [2,16] | CRITICAL |
| 2.2 | `test_tensor_network_vertex_dimension_bounds` | dimension < 2 or > 16 | Clamped to [2, 16] | CRITICAL |
| 2.3 | `test_entanglement_edge_creation_valid` | Create edge with valid params | Edge created, bond_dimension & coupling_strength constrained | CRITICAL |
| 2.4 | `test_entanglement_edge_coupling_strength_bounds` | coupling_strength > 1e6 | Clamped to 1e6 | CRITICAL |
| 2.5 | `test_tensor_network_state_add_vertex_success` | Add 1 vertex to empty state | vertex_count = 1, get_vertex returns vertex | CRITICAL |
| 2.6 | `test_tensor_network_state_add_vertex_capacity` | Add 257 vertices | 256th succeeds, 257th returns TensorNetworkFull | CRITICAL |
| 2.7 | `test_tensor_network_state_add_edge_success` | Add 1 edge with 2 vertices | edge_count = 1, incident_edges(v1) has 1 edge | CRITICAL |
| 2.8 | `test_tensor_network_state_add_edge_capacity` | Add 2049 edges | 2048th succeeds, 2049th returns TensorNetworkFull | CRITICAL |
| 2.9 | `test_tensor_network_state_add_edge_updates_indices` | Add edge updates vertex entanglement_index | source & target vertices have entanglement_index += 1 | CRITICAL |
| 2.10 | `test_tensor_network_state_serialization` | Serialize and deserialize state | JSON roundtrip preserves all fields | HIGH |

**Error Cases**:
- Adding vertex to full network (256 limit)
- Adding edge to full network (2048 limit)
- Adding edge with non-existent source/target vertices
- Negative dimensions (if not caught by type system)
- Invalid pubkey formats (empty, too long)
- Out-of-bounds coupling_strength values

---

### File: `core/src/tensor/entropy.rs`

**Module**: Tensor Network Entropy (REALMS Part IV § 3-4)  
**Public Functions**:
1. `entropy_from_cut(cut: &MinimalCut) -> TribeResult<u64>` - S(A) = |γ_A| * ln(d)
2. `mutual_information(s_a, s_b, s_union) -> TribeResult<u64>` - I(A:B) = S(A) + S(B) - S(A∪B)
3. `effective_distance(mutual_info, max_entropy) -> TribeResult<u64>` - d_eff = 1 - I(A:B)/S_max
4. `total_network_entropy(state) -> u64` - Σ entropy across all edges
5. `approximate_minimal_cut(state) -> MinimalCut` - Greedy min-cut approximation
6. `coherence_probability(local_entropy, max_entropy) -> f64` - P(unlock) = tanh(S/S_max)

**Test Targets** (Priority 1 - 8 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 3.1 | `test_entropy_from_empty_cut` | MinimalCut with 0 edges | Returns Ok(0) | CRITICAL |
| 3.2 | `test_entropy_from_single_edge` | MinimalCut with 1 edge, d=2 | ~693,000 (ln(2) * 1e6) ±100k | CRITICAL |
| 3.3 | `test_entropy_from_multiple_edges` | MinimalCut with 10 edges, various d | Proportional to sum of ln(d_i) | CRITICAL |
| 3.4 | `test_mutual_information_zero_overlap` | s_a=1e6, s_b=1e6, s_union=2e6 | Returns Ok(0) (no mutual info) | CRITICAL |
| 3.5 | `test_mutual_information_maximum_overlap` | s_a=1e6, s_b=1e6, s_union=1e6 | Returns Ok(1e6) | CRITICAL |
| 3.6 | `test_effective_distance_fully_entangled` | mutual_info=1e6, max=1e6 | Returns Ok(0) | CRITICAL |
| 3.7 | `test_effective_distance_unentangled` | mutual_info=0, max=1e6 | Returns Ok(1e6) | CRITICAL |
| 3.8 | `test_coherence_probability_bounds` | Various local/max values | Result in [0, 1] | CRITICAL |

**Edge Cases**:
- `entropy_from_cut` with empty cut
- `mutual_information` with max_entropy = 0 (division by zero)
- Negative entropy values (shouldn't occur mathematically)
- Very large bond dimensions (overflow risk)
- Very small bond dimensions (< 2, mathematical edge case)
- Saturation in tanh for `coherence_probability`

---

### File: `core/src/math/mod.rs`

**Module**: Multi-Tier Arithmetic (Portable f64, Fixed-point u64, Hardware u32)  
**Public Structs**:
1. `FixedPoint64` - u64 with configurable scale
2. `HardwareFixed` - u32 with precision bits

**Key Public Methods**:

#### FixedPoint64
```rust
pub const fn new(value: u64, scale: u32) -> Self
pub fn from_f64(f: f64, scale: u32) -> Self
pub fn to_f64(&self) -> f64
pub fn multiply(&self, other: &FixedPoint64) -> FixedPoint64
pub fn ln(&self) -> FixedPoint64
pub fn tanh(&self) -> FixedPoint64
```

#### HardwareFixed
```rust
pub const fn new(value: u32, precision_bits: u8) -> Self
pub fn multiply(&self, other: &HardwareFixed) -> HardwareFixed
pub fn ln_approx(&self) -> HardwareFixed
```

#### Portable (feature "research-precision")
```rust
pub fn ln(x: f64) -> f64
pub fn tanh(x: f64) -> f64
```

**Test Targets** (Priority 1 - 6 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 4.1 | `test_fixed_point_64_from_to_f64_roundtrip` | Create FP64 from f64, convert back | Precision loss ±1e-6, mostly reversible | CRITICAL |
| 4.2 | `test_fixed_point_64_multiply_accuracy` | 2.0 * 3.0 with scale=6 | Result ≈ 6.0 ±0.001 | CRITICAL |
| 4.3 | `test_fixed_point_64_ln_correctness` | ln(e) ≈ 1.0 | Within ±0.1 | CRITICAL |
| 4.4 | `test_fixed_point_64_tanh_bounds` | Various inputs | Output in [-1, 1] | CRITICAL |
| 4.5 | `test_hardware_fixed_multiply_no_overflow` | Max values u32::MAX | No panic, wraps safely | CRITICAL |
| 4.6 | `test_hardware_fixed_ln_approximation` | ln_approx(e) | Rough estimate, not precise | HIGH |

**Edge Cases**:
- `from_f64` with negative numbers (clamped to 0.0)
- `from_f64` with infinity/NaN
- Multiplication overflow in u64 intermediate
- Division by zero in fixed-point operations
- Scale = 0 (divide by 1)
- Very large scales (> 32 bits)
- Hardware precision_bits > 31

---

## 2. pot-o-mining Crate (Proof & Challenge Generation)

### File: `mining/src/challenge.rs`

**Module**: Challenge Generation & Management  
**Public Structs**:
1. `Challenge` - Mining challenge from Solana slot
2. `ChallengeGenerator` - Deterministic challenge factory

**Key Public Methods**:

#### Challenge
```rust
pub fn is_expired(&self) -> bool
pub fn to_mining_task(&self, requester: &str) -> MiningTask
pub id: String
pub slot: u64
pub slot_hash: String
pub operation_type: String  // "matrix_multiply", "relu", etc.
pub difficulty: u64
pub mml_threshold: f64
pub path_distance_max: u32
```

#### ChallengeGenerator
```rust
pub fn new(difficulty: u64, max_tensor_dim: usize) -> Self
pub fn generate(&self, slot: u64, slot_hash_hex: &str) -> TribeResult<Challenge>
pub fn base_difficulty: u64
pub fn base_mml_threshold: f64
pub fn challenge_ttl_secs: i64
```

**Test Targets** (Priority 1 - 8 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 5.1 | `test_challenge_generation_valid_hash` | 64-char hex slot_hash | Challenge created with all fields populated | CRITICAL |
| 5.2 | `test_challenge_generation_invalid_hex` | Non-hex characters in hash | Returns TribeError::InvalidOperation | CRITICAL |
| 5.3 | `test_challenge_is_expired` | Create challenge, check expiry | is_expired() false initially, true after TTL | CRITICAL |
| 5.4 | `test_challenge_to_mining_task` | Convert challenge to task | MiningTask has correct operation_type, difficulty | CRITICAL |
| 5.5 | `test_challenge_generator_deterministic` | Same slot/hash twice | Produces identical challenges (id, operation_type, difficulty) | CRITICAL |
| 5.6 | `test_challenge_difficulty_scaling` | Slot 0 vs slot 100,000 | Higher slots have higher difficulty | CRITICAL |
| 5.7 | `test_challenge_mml_threshold_scaling` | Difficulty 1 vs 16 | Higher difficulty has lower threshold | CRITICAL |
| 5.8 | `test_challenge_operation_type_deterministic` | Hash with byte 0 = 0xFF | Same operation selected for same hash | CRITICAL |

**Edge Cases**:
- Invalid hex input (odd length, invalid chars)
- Empty slot_hash string
- Very large slot numbers (> u64::MAX)
- Very short hashes (< 1 byte)
- Very long hashes (> 1000 chars)
- Slot hash with leading zeros
- Challenge TTL = 0 (immediate expiry)
- max_tensor_dim = 1 (should be >= 2)

---

### File: `mining/src/pot_o.rs`

**Module**: PoT-O Consensus Engine (CRITICAL)  
**Public Structs**:
1. `PotOProof` - Full proof submitted by miner
2. `PotOConsensus` - Consensus engine orchestrator

**Key Public Methods**:

#### PotOConsensus
```rust
pub fn new(difficulty: u64, max_tensor_dim: usize) -> Self
pub fn generate_challenge(&self, slot: u64, slot_hash: &str) -> TribeResult<Challenge>
pub fn mine(&self, challenge, miner_pubkey, max_iterations) -> TribeResult<Option<PotOProof>>
pub fn verify_proof(&self, proof, challenge) -> TribeResult<bool>
pub fn expected_paths_and_calcs(&self, challenge) -> (u64, u64)
pub fn engine_stats(&self) -> EngineStats
pub fn compute_proof_hash(...) -> String  // Deterministic hash
```

**Test Targets** (Priority 1 - 10 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 6.1 | `test_consensus_generate_challenge_valid` | Generate from valid slot_hash | Challenge returned with all fields | CRITICAL |
| 6.2 | `test_consensus_mine_finds_proof_low_difficulty` | Mine with difficulty=1, max_iter=1000 | Option<PotOProof> is Some | CRITICAL |
| 6.3 | `test_consensus_mine_returns_none_high_difficulty` | Mine with difficulty=100, max_iter=10 | Option<PotOProof> is None (not found) | CRITICAL |
| 6.4 | `test_consensus_verify_valid_proof` | Mine proof, verify same proof | verify_proof returns Ok(true) | CRITICAL |
| 6.5 | `test_consensus_verify_tampered_proof` | Mine proof, modify tensor_hash, verify | verify_proof returns Ok(false) | CRITICAL |
| 6.6 | `test_consensus_verify_proof_hash_integrity` | Compute proof hash twice same inputs | Hashes identical (deterministic) | CRITICAL |
| 6.7 | `test_consensus_expected_paths_and_calcs` | Check expected values | expected_paths > 0, expected_calcs = 1 + difficulty | CRITICAL |
| 6.8 | `test_consensus_engine_stats_tracking` | Mine multiple proofs | stats.successful_tasks increments | HIGH |
| 6.9 | `test_consensus_proof_computation_time_recorded` | Mine proof, check stats | average_task_time > 0 | HIGH |
| 6.10 | `test_consensus_mml_score_validation` | Mine with low mml_threshold | Rejects proofs above threshold | HIGH |

**Error Cases**:
- Invalid slot hash format
- Mining timeout (max_iterations reached)
- Invalid miner pubkey format
- Tensor computation failure
- MML score exceeds threshold
- Neural path distance exceeds max
- Proof hash mismatch
- Missing input tensors

---

### File: `mining/src/neural_path.rs`

**Module**: Neural Path Validation  
**Public Struct**: `NeuralPathValidator`

**Key Public Methods**:
```rust
pub fn expected_path_signature(&self, challenge_hash: &str) -> Vec<u8>  // Deterministic
pub fn compute_actual_path(&self, tensor: &Tensor, nonce: u64) -> TribeResult<Vec<u8>>
pub fn hamming_distance(a: &[u8], b: &[u8]) -> u32
pub fn validate(&self, actual_path, challenge_hash, max_distance) -> bool
pub fn path_to_hex(path: &[u8]) -> String
pub layer_widths: Vec<usize>  // [32, 16, 8]
```

**Test Targets** (Priority 1 - 8 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 7.1 | `test_neural_path_expected_deterministic` | Call twice with same hash | Returns identical Vec<u8> | CRITICAL |
| 7.2 | `test_neural_path_compute_actual_varies_with_nonce` | Different nonces on same tensor | Usually different paths (>50% different bits) | CRITICAL |
| 7.3 | `test_neural_path_hamming_distance_identical` | Distance between [0,1,0] and [0,1,0] | Returns 0 | CRITICAL |
| 7.4 | `test_neural_path_hamming_distance_different` | Distance between [0,1,0] and [1,0,1] | Returns 3 | CRITICAL |
| 7.5 | `test_neural_path_validate_matching` | Path within max_distance of expected | Returns true | CRITICAL |
| 7.6 | `test_neural_path_validate_nonmatching` | Path outside max_distance | Returns false | CRITICAL |
| 7.7 | `test_neural_path_path_to_hex_roundtrip` | 8 bits -> hex -> back | Preserves bit pattern | HIGH |
| 7.8 | `test_neural_path_layer_widths_total_bits` | Compute for default layers | total_bits = 32 + 16 + 8 = 56 | HIGH |

**Edge Cases**:
- Empty tensor (0 elements)
- Single-element tensor
- Very large tensor (exceeds layer widths)
- Challenge hash that's empty string
- Challenge hash with invalid hex
- Nonce = 0
- Nonce = u64::MAX
- Actual path shorter than expected
- Actual path longer than expected

---

### File: `mining/src/mml_path.rs`

**Module**: Minimum Message Length (MML) Validation  
**Public Struct**: `MMLPathValidator`

**Key Public Methods**:
```rust
pub fn compute_mml_score(&self, input: &Tensor, output: &Tensor) -> TribeResult<f64>
pub fn compute_entropy_mml_score(&self, input: &Tensor, output: &Tensor) -> f64
pub fn validate(&self, mml_score: f64, mml_threshold: f64) -> bool
pub fn threshold_for_difficulty(base_threshold: f64, difficulty: u64) -> f64
pub compression_level: u32  // 6 default
```

**Test Targets** (Priority 1 - 6 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 8.1 | `test_mml_score_zeros_compresses_better` | Compare [1,2,3,4] vs [0,0,0,0] | zeros_score < ones_score | CRITICAL |
| 8.2 | `test_mml_score_identical_input_output` | Same tensor input=output | score ≈ 1.0 | CRITICAL |
| 8.3 | `test_mml_validate_below_threshold` | score=0.5, threshold=0.8 | Returns true | CRITICAL |
| 8.4 | `test_mml_validate_above_threshold` | score=0.9, threshold=0.8 | Returns false | CRITICAL |
| 8.5 | `test_mml_threshold_scaling_with_difficulty` | diff=1 vs diff=16 | difficulty=16 has lower threshold | CRITICAL |
| 8.6 | `test_mml_entropy_score_non_zero` | Any non-trivial tensors | score > 0.0 | HIGH |

**Error Cases**:
- Empty input tensor
- Empty output tensor
- Mismatched tensor shapes (should still compute ratio)
- Zero-entropy inputs (all same bytes)
- Compression errors (should handle gracefully)
- Very large tensors (memory constraints)
- Invalid compression level values

---

## 3. pot-o-extensions Crate (Device & Network Protocol)

### File: `extensions/src/device_protocol.rs`

**Module**: Device Protocol Abstraction  
**Public Trait & Implementations**:
```rust
pub trait DeviceProtocol: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn max_tensor_dims(&self) -> (usize, usize);
    fn max_working_memory(&self) -> usize;
    fn heartbeat(&self) -> TribeResult<DeviceStatus>;
    fn supported_operations(&self) -> Vec<&'static str>;
}
```

**Implementations**:
1. `NativeDevice` - (1024x1024 tensor, 512MB mem)
2. `ESP32SDevice` - (64x64 tensor, 320KB mem, heartbeat timeout tracking)
3. `ESP8266Device` - (32x32 tensor, 80KB mem)
4. `WASMDevice` - (TBD)

**Test Targets** (Priority 1 - 8 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 9.1 | `test_native_device_trait_impl` | Create NativeDevice, call trait methods | DeviceType::Native, max 1024x1024, 512MB | CRITICAL |
| 9.2 | `test_native_device_heartbeat` | Call heartbeat() | DeviceStatus online=true, uptime >= 0 | CRITICAL |
| 9.3 | `test_esp32s_device_creation` | Create ESP32SDevice("id_123") | device_id stored, max 64x64, 320KB | CRITICAL |
| 9.4 | `test_esp32s_device_record_heartbeat` | Call record_heartbeat multiple times | last_seen updates, is_stale reflects time | CRITICAL |
| 9.5 | `test_esp32s_device_is_stale_timeout` | Record heartbeat, wait, check stale | is_stale(90) = false initially, true after 90s | CRITICAL |
| 9.6 | `test_esp32s_device_heartbeat_status` | Call heartbeat(), check status | Returns DeviceStatus with online, uptime | HIGH |
| 9.7 | `test_device_protocol_supported_operations` | Check supported_operations() | Native supports all 7, ESP32S missing tanh | HIGH |
| 9.8 | `test_device_type_enum_variants` | Enumerate DeviceType | All 5 variants (Native, ESP32S, ESP8266, WASM, Custom) | HIGH |

**Edge Cases**:
- Device ID with special characters (spaces, unicode)
- Device ID empty string
- max_tensor_dims asymmetric
- Negative max_working_memory (shouldn't occur)
- Heartbeat called repeatedly in quick succession
- Device offline/offline transitions
- Timeout boundary conditions (exactly 90s)
- Very large uptime values (overflow)

---

### File: `extensions/src/chain_bridge.rs`

**Module**: Chain Bridge (Solana & EVM)  
**Public Trait & Implementations**:
```rust
#[async_trait]
pub trait ChainBridge: Send + Sync {
    async fn submit_proof(&self, proof: &ProofPayload) -> TribeResult<TxSignature>;
    async fn query_miner(&self, pubkey: &str) -> TribeResult<Option<MinerAccount>>;
    async fn register_miner(&self, miner_pubkey: &str) -> TribeResult<TxSignature>;
    async fn get_current_difficulty(&self) -> TribeResult<u64>;
    async fn request_swap(&self, from: Token, to: Token, amount: u64) -> TribeResult<TxSignature>;
}
```

**Implementations**:
1. `SolanaBridge` - Solana RPC integration

**Test Targets** (Priority 1 - 6 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 10.1 | `test_solana_bridge_creation_valid_program_id` | Create with valid Pubkey | Bridge created, program_id set | CRITICAL |
| 10.2 | `test_solana_bridge_creation_invalid_program_id` | Invalid program_id string | Uses default Pubkey, logs warning | CRITICAL |
| 10.3 | `test_solana_bridge_creation_missing_keypair` | Keypair file not found | relayer_keypair = None, logs warning | HIGH |
| 10.4 | `test_solana_bridge_hex_to_32_valid` | "abcd...ef" (64 hex chars) | Returns [u8; 32] | HIGH |
| 10.5 | `test_solana_bridge_hex_to_32_invalid` | "xyz..." (invalid hex) | Returns TribeError::ChainBridgeError | HIGH |
| 10.6 | `test_anchor_discriminator_deterministic` | Same name twice | Returns identical [u8; 8] | HIGH |

**Error Cases**:
- Invalid program ID format
- Missing keypair file
- Corrupted keypair file
- Invalid hex in proof hash
- RPC connection failure
- Miner not found on-chain
- Insufficient account funds
- Transaction serialization errors

---

### File: `extensions/src/security.rs`

**Module**: Proof Authority & Node Authentication  
**Public Trait & Implementations**:
```rust
pub trait ProofAuthority: Send + Sync {
    fn verify_miner_identity(&self, pubkey: &str, signature: &[u8]) -> TribeResult<bool>;
    fn sign_challenge(&self, challenge: &Challenge) -> TribeResult<Vec<u8>>;
    fn validate_node_connection(&self, peer: &PeerInfo) -> TribeResult<bool>;
}
```

**Implementations**:
1. `Ed25519Authority` - Solana keypair based (local-mode stub)
2. `MtlsAuthority` - mTLS for VPN (stubbed for future)
3. `HmacDeviceAuth` - HMAC for ESP devices (stubbed for future)

**Test Targets** (Priority 1 - 6 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 11.1 | `test_ed25519_authority_verify_identity` | Call verify_miner_identity | Returns Ok(true) (local mode accepts all) | CRITICAL |
| 11.2 | `test_ed25519_authority_sign_challenge` | Call sign_challenge | Returns Ok(vec![0; 64]) (placeholder) | CRITICAL |
| 11.3 | `test_ed25519_authority_validate_connection` | Call validate_node_connection | Returns Ok(true) (local mode) | CRITICAL |
| 11.4 | `test_mtls_authority_creation` | Create MtlsAuthority | Struct created with config | HIGH |
| 11.5 | `test_hmac_device_auth_creation` | Create HmacDeviceAuth with secret | Struct created with shared_secret | HIGH |
| 11.6 | `test_proof_authority_trait_object` | Create Box<dyn ProofAuthority> | Trait object works polymorphically | HIGH |

**Edge Cases**:
- Empty pubkey string
- Empty signature bytes
- Very long pubkey (1000+ chars)
- Very long signature (1MB+)
- Challenge with missing fields
- PeerInfo with empty node_id
- All unimplemented (MtlsAuthority, HmacDeviceAuth) should panic with todo!()

---

### File: `extensions/src/peer_network.rs`

**Module**: Peer Network (Local & VPN Mesh)  
**Public Trait & Implementations**:
```rust
#[async_trait]
pub trait PeerNetwork: Send + Sync {
    fn node_id(&self) -> &NodeId;
    async fn discover_peers(&self) -> TribeResult<Vec<PeerInfo>>;
    async fn broadcast_challenge(&self, challenge: &Challenge) -> TribeResult<()>;
    async fn relay_proof(&self, proof: &ProofPayload) -> TribeResult<()>;
    async fn sync_state(&self) -> TribeResult<NetworkState>;
}
```

**Implementations**:
1. `LocalOnlyNetwork` - Single node, no-op for all methods
2. `VpnMeshNetwork` - WireGuard + mDNS (stubbed for future)

**Test Targets** (Priority 1 - 6 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 12.1 | `test_local_only_network_creation` | Create LocalOnlyNetwork | node_id set to UUID | CRITICAL |
| 12.2 | `test_local_only_network_node_id_unique` | Create two networks | Different UUIDs | CRITICAL |
| 12.3 | `test_local_only_network_discover_peers` | Call discover_peers | Returns Ok(vec![]) | CRITICAL |
| 12.4 | `test_local_only_network_broadcast_noop` | Call broadcast_challenge | Returns Ok(()), no-op | CRITICAL |
| 12.5 | `test_local_only_network_relay_proof_noop` | Call relay_proof | Returns Ok(()), no-op | CRITICAL |
| 12.6 | `test_local_only_network_sync_state` | Call sync_state | Returns NetworkState with peers=[], synced=true | CRITICAL |

**Edge Cases**:
- VpnMeshNetwork creation (not yet implemented)
- PeerInfo with invalid address
- Challenge relay on network with 0 peers
- Broadcast during network outage (simulated)

---

## 4. ai3-lib Crate (Tensor Engine & ESP Compatibility)

### File: `ai3-lib/src/tensor.rs`

**Module**: Tensor Types & Operations  
**Public Structs**:
1. `TensorShape` - Dimension sizes
2. `TensorData` - Element data (F32 or U8)
3. `Tensor` - Complete tensor

**Key Public Methods**:

#### TensorShape
```rust
pub fn new(dims: Vec<usize>) -> Self
pub fn total_elements(&self) -> usize
pub fn is_matrix(&self) -> bool
```

#### TensorData
```rust
pub fn len(&self) -> usize
pub fn is_empty(&self) -> bool
pub fn as_f32(&self) -> Vec<f32>
pub fn to_bytes(&self) -> Vec<u8>
```

#### Tensor (CRITICAL)
```rust
pub fn new(shape, data) -> TribeResult<Self>  // Validates shape vs data
pub fn zeros(shape) -> Self
pub fn from_slot_hash(hash_bytes) -> Self
pub fn calculate_hash(&self) -> String
pub fn clamp_dimensions(&self, max_dim: usize) -> Self
pub fn byte_size(&self) -> usize
```

**Test Targets** (Priority 1 - 10 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 13.1 | `test_tensor_shape_creation_valid` | Create TensorShape([8, 8]) | dims=[8,8], total_elements=64 | CRITICAL |
| 13.2 | `test_tensor_shape_total_elements_calculation` | TensorShape([2, 3, 4]) | total_elements() = 24 | CRITICAL |
| 13.3 | `test_tensor_shape_is_matrix` | [8, 8] vs [8, 8, 3] vs [64] | is_matrix() = true, false, false | CRITICAL |
| 13.4 | `test_tensor_data_f32_len` | TensorData::F32(vec![1.0; 64]) | len() = 64 | CRITICAL |
| 13.5 | `test_tensor_data_u8_to_f32_conversion` | U8([0, 127, 255]) | as_f32() = [0.0, 0.498, 1.0] ±0.01 | CRITICAL |
| 13.6 | `test_tensor_new_valid` | new(TensorShape([4]), F32([1,2,3,4])) | Tensor created | CRITICAL |
| 13.7 | `test_tensor_new_shape_mismatch` | new(TensorShape([4]), F32([1,2,3])) | Returns TribeError::TensorError | CRITICAL |
| 13.8 | `test_tensor_zeros` | zeros(TensorShape([3, 3])) | All elements 0.0, shape correct | CRITICAL |
| 13.9 | `test_tensor_clamp_dimensions_reduces_size` | clamp_dimensions(32) on 64x64 | Result max dim = 32 | CRITICAL |
| 13.10 | `test_tensor_calculate_hash_deterministic` | Hash same tensor twice | Identical hashes | HIGH |

**Edge Cases**:
- Empty shape (dims = [])
- Shape with 0 in any dimension
- Very large shapes (> 1000x1000)
- Single element tensors
- 1D vs 2D vs 3D tensors
- U8([0]) and U8([255]) edge cases
- Clamping when already small
- Hash of empty tensor
- byte_size with very large tensors

---

### File: `ai3-lib/src/operations.rs`

**Module**: Tensor Operations (TensorOp trait + implementations)  
**Public Trait & Operations**:
```rust
pub trait TensorOp: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, input: &Tensor) -> TribeResult<Tensor>;
}

pub fn parse_operation(op_type: &str) -> TribeResult<Box<dyn TensorOp>>
```

**Implementations**:
1. `MatrixMultiply` - Self-multiplication
2. `Convolution` - 1D convolution with fixed kernel
3. `ActivationFunction::ReLU` - max(0, x)
4. `ActivationFunction::Sigmoid` - 1/(1+e^-x)
5. `ActivationFunction::Tanh` - tanh(x)
6. `VectorOp::DotProduct` - Dot product of two halves
7. `VectorOp::Normalize` - L2 normalization

**Test Targets** (Priority 1 - 10 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 14.1 | `test_parse_operation_all_types` | Parse all 7 operation types | All return Ok(Box<dyn TensorOp>) | CRITICAL |
| 14.2 | `test_parse_operation_invalid` | Parse "unknown_op" | Returns TribeError::TensorError | CRITICAL |
| 14.3 | `test_matrix_multiply_identity` | 2x2 identity matrix × itself | Result ≈ identity | CRITICAL |
| 14.4 | `test_convolution_smoothing` | Kernel [0.25, 0.5, 0.25] on [1,2,3] | Smoothed output | CRITICAL |
| 14.5 | `test_convolution_short_input` | Input shorter than kernel | Returns input unchanged | HIGH |
| 14.6 | `test_relu_activation` | Input [-2, -1, 0, 1, 2] | Output [0, 0, 0, 1, 2] | CRITICAL |
| 14.7 | `test_sigmoid_bounds` | Various inputs | Output in (0, 1) for most inputs | CRITICAL |
| 14.8 | `test_tanh_bounds` | Various inputs | Output in (-1, 1) | CRITICAL |
| 14.9 | `test_dot_product_zero_vectors` | [0,0,0] dot [0,0,0] | Result = 0.0 | CRITICAL |
| 14.10 | `test_normalize_unit_vector` | Already normalized vector | Output ≈ same | HIGH |

**Edge Cases**:
- MatrixMultiply on non-square tensors
- MatrixMultiply with 1x1 matrix
- MatrixMultiply with empty matrix
- Convolution with kernel longer than input
- Convolution with empty kernel
- Activation functions on NaN/Inf inputs
- DotProduct on odd-length tensors
- Normalize on zero vector
- Normalize on very large magnitude vectors

---

### File: `ai3-lib/src/esp_compat.rs`

**Module**: ESP32/ESP8266 Compatibility  
**Public Types**:
```rust
pub enum ESPDeviceType { ESP32, ESP32S, ESP8266 }
pub struct ESPMiningConfig { ... }
pub struct ESPCompatibility;  // Utility functions
```

**Key Public Methods**:
```rust
impl ESPDeviceType {
    pub fn max_tensor_dims(&self) -> (usize, usize)  // ESP32: 64x64, ESP8266: 32x32
    pub fn max_working_memory(&self) -> usize  // ESP32: 320KB, ESP8266: 80KB
    pub fn supported_operations(&self) -> Vec<&'static str>
}

impl ESPCompatibility {
    pub fn get_recommended_config(device_type) -> ESPMiningConfig
    pub fn fits_device(tensor, device_type) -> bool
    pub fn optimize_for_esp(tensor, device_type) -> TribeResult<Tensor>
    pub fn most_restrictive_dim(devices) -> usize
}
```

**Test Targets** (Priority 1 - 8 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 15.1 | `test_esp_device_type_max_tensor_dims` | ESP32 vs ESP32S vs ESP8266 | 64x64, 64x64, 32x32 respectively | CRITICAL |
| 15.2 | `test_esp_device_type_max_working_memory` | All three device types | 320KB, 320KB, 80KB | CRITICAL |
| 15.3 | `test_esp_device_type_supported_operations` | ESP32 vs ESP8266 | ESP32: 6 ops, ESP8266: 4 ops | CRITICAL |
| 15.4 | `test_esp_device_type_from_str` | "esp32" vs "ESP8266" vs "unknown" | Parses correctly, case-insensitive | CRITICAL |
| 15.5 | `test_esp_mining_config_for_device` | Create config for ESP32S | max_tensor_dim=64, rpc_port=8900 | CRITICAL |
| 15.6 | `test_esp_compatibility_fits_device` | 64x64 tensor on ESP32 vs ESP8266 | true vs false | CRITICAL |
| 15.7 | `test_esp_compatibility_optimize_for_esp` | Large 1024x1024 tensor on ESP8266 | Clamped to 32x32 | CRITICAL |
| 15.8 | `test_esp_compatibility_most_restrictive_dim` | Mix of [ESP32, ESP8266] | Returns 32 | CRITICAL |

**Edge Cases**:
- Invalid device type strings
- Tensor exactly at memory limit
- Tensor 1 byte over memory limit
- Device list with duplicates
- Device list with only ESP32S (all same)
- Empty device list
- Very large tensor (> 1MB)
- Tensor with 1 dimension vs 2D vs 3D

---

### File: `ai3-lib/src/mining.rs`

**Module**: Mining Tasks & Task Distribution  
**Public Structs**:
1. `MiningTask` - Single mining task
2. `MiningResult` - Task completion result
3. `MinerCapabilities` - Miner capabilities
4. `MinerStats` - Miner statistics
5. `TaskDistributor` - Task distribution

**Key Public Methods**:

#### MiningTask
```rust
pub fn new(operation_type, input_tensors, difficulty, reward, max_time, requester) -> Self
pub fn is_expired(&self) -> bool
pub fn meets_difficulty(&self, hash: &str) -> bool  // Checks leading zeros
```

#### TaskDistributor
```rust
pub fn new() -> Self
pub fn add_task(&mut self, task: MiningTask)
pub fn get_pending_tasks(&self) -> Vec<&MiningTask>
pub fn remove_task(&mut self, task_id: &str) -> Option<MiningTask>
pub fn cleanup_expired_tasks(&mut self)
```

**Test Targets** (Priority 1 - 8 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 16.1 | `test_mining_task_creation_valid` | Create MiningTask with valid params | id set, created_at set, expires_at = now + max_time | CRITICAL |
| 16.2 | `test_mining_task_is_expired_false` | Create task, check immediately | is_expired() = false | CRITICAL |
| 16.3 | `test_mining_task_meets_difficulty_zeros` | Hash "0000abc..." vs difficulty 4 | meets_difficulty() = true | CRITICAL |
| 16.4 | `test_mining_task_meets_difficulty_fails` | Hash "00abc..." vs difficulty 4 | meets_difficulty() = false | CRITICAL |
| 16.5 | `test_task_distributor_add_get_remove` | Add 3 tasks, get, remove 1 | Correct task management | CRITICAL |
| 16.6 | `test_task_distributor_cleanup_expired` | Add 3 tasks, expire 2, cleanup | Only 1 task remains | CRITICAL |
| 16.7 | `test_miner_capabilities_default` | Create default MinerCapabilities | Supports all 7 operations, 64x64 max | HIGH |
| 16.8 | `test_miner_stats_default` | Create default MinerStats | All counts = 0 | HIGH |

**Edge Cases**:
- MiningTask with max_computation_time = 0
- MiningTask with empty operation_type
- MiningTask with empty input_tensors
- Hash with mixed case hex
- Hash shorter/longer than expected
- Multiple difficulty checks (0, 1, 10, 64 leading zeros)
- Remove task that doesn't exist
- Cleanup when all expired / none expired
- Add duplicate task IDs

---

### File: `ai3-lib/src/lib.rs`

**Module**: AI3 Engine (Main tensor engine)  
**Public Structs**:
1. `AI3Engine` - Main tensor execution engine
2. `EngineConfig` - Engine configuration
3. `EngineStats` - Engine statistics

**Key Public Methods**:
```rust
impl AI3Engine {
    pub fn new() -> Self
    pub fn with_config(config: EngineConfig) -> Self
    pub fn execute_task(&self, task: &MiningTask) -> TribeResult<Tensor>
    pub fn get_stats(&self) -> EngineStats
    pub fn record_result(&self, success: bool, duration: Duration)
}

pub trait TensorEngine: Send + Sync {
    fn execute_task(&self, task: &MiningTask) -> TribeResult<Tensor>;
    fn get_stats(&self) -> EngineStats;
    fn record_result(&self, success: bool, duration: Duration);
}
```

**Test Targets** (Priority 1 - 6 tests):

| # | Test Name | Scenario | Expected Behavior | Priority |
|---|-----------|----------|-------------------|----------|
| 17.1 | `test_ai3_engine_creation_default` | Create AI3Engine::new() | Engine ready to execute tasks | CRITICAL |
| 17.2 | `test_ai3_engine_execute_task_matrix_multiply` | Execute matrix_multiply task | Returns Ok(Tensor) with result | CRITICAL |
| 17.3 | `test_ai3_engine_execute_task_relu` | Execute relu task | Returns Ok(Tensor) with activations | CRITICAL |
| 17.4 | `test_ai3_engine_execute_task_invalid_op` | Execute unknown operation | Returns TribeError::TensorError | CRITICAL |
| 17.5 | `test_ai3_engine_stats_tracking` | Execute 5 tasks, get stats | stats.total_tasks_processed = 5 | CRITICAL |
| 17.6 | `test_ai3_engine_trait_object` | Box<dyn TensorEngine> | Trait object works polymorphically | HIGH |

**Edge Cases**:
- Engine with max_concurrent_tasks = 0
- Engine with very short timeout (1ms)
- Execute task without input tensors
- Execute task with multiple input tensors
- Record result with very large duration
- Stats on zero executed tasks

---

# PRIORITY 2 (HIGH) - Additional Coverage Tests

**Summary**: These are important tests for robustness but not blocking core functionality.

## Additional Edge Case & Integration Tests

### Cross-Module Integration Tests

| # | Test Name | Modules Involved | Scenario | Priority |
|---|-----------|-------------------|----------|----------|
| 18.1 | `test_e2e_challenge_to_proof` | mining (challenge, pot_o), ai3 (engine), core (types) | Generate challenge → mine → verify proof | HIGH |
| 18.2 | `test_e2e_tensor_entropy_calculation` | core (tensor, types), ai3 (tensor) | Create network → calculate entropy | HIGH |
| 18.3 | `test_e2e_mml_and_neural_validation` | mining (mml, neural), ai3 (ops) | MML score + neural path matching | HIGH |
| 18.4 | `test_device_to_chain_bridge_integration` | extensions (device, chain_bridge), mining (proof) | Device registers → submits proof | HIGH |
| 18.5 | `test_esp_tensor_constraints` | ai3 (esp_compat, tensor, ops), mining (challenge) | ESP device receives challenge → compute within limits | HIGH |

### Error Propagation Tests

| # | Test Name | Error Path | Verification | Priority |
|---|-----------|-----------|--------------|----------|
| 19.1 | `test_error_propagation_invalid_challenge` | challenge.rs → pot_o.rs → verify | TribeError flows through stack | HIGH |
| 19.2 | `test_error_propagation_tensor_operation` | operations.rs → mining.rs → validate | Tensor errors logged properly | HIGH |
| 19.3 | `test_error_propagation_device_offline` | device_protocol.rs → chain_bridge.rs | Device error → chain bridge error | HIGH |
| 19.4 | `test_error_propagation_network_failure` | peer_network.rs → chain_bridge.rs | Network error → bridge error | HIGH |

### Performance & Stress Tests

| # | Test Name | Scenario | Metric | Priority |
|---|-----------|----------|--------|----------|
| 20.1 | `test_tensor_operations_performance` | 1000 ops in sequence | < 5s total | MEDIUM |
| 20.2 | `test_network_state_capacity_stress` | Add 256 vertices + 2048 edges | No panics, all succeed | MEDIUM |
| 20.3 | `test_mining_task_distributor_throughput` | Add/remove 10k tasks | < 1s total | MEDIUM |
| 20.4 | `test_entropy_calculation_large_cut` | 1000-edge minimal cut | Completes in < 100ms | MEDIUM |

---

# Test Organization & File Structure

## Recommended Test Layout

```
pot-o-validator/
├── core/
│   ├── src/
│   │   ├── error.rs
│   │   ├── types/
│   │   ├── tensor/
│   │   └── math/
│   └── tests/
│       ├── test_error.rs           (4 tests)
│       ├── test_tensor_network.rs  (10 tests)
│       ├── test_entropy.rs         (8 tests)
│       ├── test_math.rs            (6 tests)
│       └── integration/
│           └── test_core_e2e.rs
│
├── mining/
│   ├── src/
│   │   ├── challenge.rs
│   │   ├── pot_o.rs
│   │   ├── neural_path.rs
│   │   └── mml_path.rs
│   └── tests/
│       ├── test_challenge.rs       (8 tests)
│       ├── test_pot_o.rs           (10 tests)
│       ├── test_neural_path.rs     (8 tests)
│       ├── test_mml_path.rs        (6 tests)
│       └── integration/
│           └── test_mining_e2e.rs
│
├── extensions/
│   ├── src/
│   │   ├── device_protocol.rs
│   │   ├── chain_bridge.rs
│   │   ├── security.rs
│   │   └── peer_network.rs
│   └── tests/
│       ├── test_device_protocol.rs (8 tests)
│       ├── test_chain_bridge.rs    (6 tests)
│       ├── test_security.rs        (6 tests)
│       ├── test_peer_network.rs    (6 tests)
│       └── integration/
│           └── test_extensions_e2e.rs
│
├── ai3-lib/
│   ├── src/
│   │   ├── tensor.rs
│   │   ├── operations.rs
│   │   ├── esp_compat.rs
│   │   ├── mining.rs
│   │   └── lib.rs
│   └── tests/
│       ├── test_tensor.rs          (10 tests)
│       ├── test_operations.rs      (10 tests)
│       ├── test_esp_compat.rs      (8 tests)
│       ├── test_mining.rs          (8 tests)
│       ├── test_ai3_engine.rs      (6 tests)
│       └── integration/
│           └── test_ai3_e2e.rs
│
└── tests/
    ├── integration/
    │   └── test_e2e_workflow.rs    (5 integration tests)
    └── error_propagation/
        └── test_error_flow.rs      (4 error flow tests)
```

---

# Test Execution Roadmap

## Phase Timeline

| Phase | Target | Tests | Effort | Timeline |
|-------|--------|-------|--------|----------|
| **2.9** | pot-o-core | 28 tests | 4-5 days | Week 1 |
| **2.10** | pot-o-mining | 32 tests | 4-5 days | Week 2 |
| **2.11** | pot-o-extensions | 26 tests | 3-4 days | Week 3 |
| **2.12** | ai3-lib | 42 tests | 4-5 days | Week 4 |
| **2.13** | Integration | 9 tests | 2-3 days | Week 5 |
| **2.14** | Optimization | Refactor | 1-2 days | Week 5 |
| **2.15** | Documentation | Results | 1 day | Week 6 |

**Total Effort**: ~20-25 days  
**Target Completion**: 60-65 total tests  
**Estimated Coverage**: 45-50% of codebase

---

# Summary Table: All Test Targets

| # | Crate | Module | Test Count | Priority | Status |
|---|-------|--------|-----------|----------|--------|
| 1 | core | error.rs | 4 | CRITICAL | ❌ Pending |
| 2 | core | tensor_network.rs | 10 | CRITICAL | ❌ Pending |
| 3 | core | entropy.rs | 8 | CRITICAL | ❌ Pending |
| 4 | core | math/mod.rs | 6 | CRITICAL | ❌ Pending |
| 5 | mining | challenge.rs | 8 | CRITICAL | ❌ Pending |
| 6 | mining | pot_o.rs | 10 | CRITICAL | ❌ Pending |
| 7 | mining | neural_path.rs | 8 | CRITICAL | ❌ Pending |
| 8 | mining | mml_path.rs | 6 | CRITICAL | ❌ Pending |
| 9 | extensions | device_protocol.rs | 8 | CRITICAL | ❌ Pending |
| 10 | extensions | chain_bridge.rs | 6 | CRITICAL | ❌ Pending |
| 11 | extensions | security.rs | 6 | CRITICAL | ❌ Pending |
| 12 | extensions | peer_network.rs | 6 | CRITICAL | ❌ Pending |
| 13 | ai3-lib | tensor.rs | 10 | CRITICAL | ❌ Pending |
| 14 | ai3-lib | operations.rs | 10 | CRITICAL | ❌ Pending |
| 15 | ai3-lib | esp_compat.rs | 8 | CRITICAL | ❌ Pending |
| 16 | ai3-lib | mining.rs | 8 | CRITICAL | ❌ Pending |
| 17 | ai3-lib | lib.rs | 6 | CRITICAL | ❌ Pending |
| 18-22 | mixed | Integration tests | 9 | HIGH | ❌ Pending |
| **TOTAL** | **All** | **All** | **137** | **Mixed** | **❌ All Pending** |

---

## Key Success Metrics

### Acceptance Criteria
- ✅ All 137 tests compile and run without warnings
- ✅ All tests pass on main branch
- ✅ No flaky tests (pass 100/100 runs)
- ✅ Minimum 45% code coverage
- ✅ All error cases tested
- ✅ All edge cases documented

### Coverage Targets by Module
- `error.rs`: 95%+ (comprehensive error testing)
- `tensor_network.rs`: 90%+ (state machine critical)
- `entropy.rs`: 85%+ (mathematical validation)
- `math/mod.rs`: 80%+ (multiple backends)
- `challenge.rs`: 85%+ (determinism critical)
- `pot_o.rs`: 85%+ (core consensus)
- `neural_path.rs`: 80%+ (path validation)
- `mml_path.rs`: 80%+ (compression validation)
- `device_protocol.rs`: 75%+ (trait implementations)
- `chain_bridge.rs`: 70%+ (async trait, RPC mocking)
- `tensor.rs`: 90%+ (data structure critical)
- `operations.rs`: 85%+ (computation validation)
- `esp_compat.rs`: 80%+ (hardware constraints)

---

## Notes on Test Implementation

### Async Tests
Use `#[tokio::test]` for async functions in:
- chain_bridge.rs (async trait methods)
- peer_network.rs (async trait methods)

### Mocking Strategy
- Mock RPC calls in SolanaBridge tests
- Mock file I/O for keypair loading
- Mock system time for heartbeat tests
- Use in-memory state for device protocol tests

### Determinism
- Use fixed random seeds for reproducible tests
- Document non-deterministic behaviors
- Test against known golden values
- Use property-based testing for mathematical functions

### Performance Considerations
- Set reasonable timeouts for long-running tests
- Profile critical paths (entropy calculation, mining)
- Document performance baselines

---

**End of Analysis Document**

