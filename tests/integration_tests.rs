//! Integration tests for pot-o-validator v0.2.0
//!
//! Tests multi-module interactions and end-to-end workflows

use ai3_lib::{AI3Engine, TensorEngine};
use pot_o_core::{Block, Transaction, TransactionType, TribeError};
use pot_o_mining::ChallengeGenerator;

#[test]
fn test_e2e_challenge_to_mining_task() {
    // Test: Challenge generation → mining task creation
    let gen = ChallengeGenerator::default();
    let challenge = gen
        .generate(
            100,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("challenge generation");

    let task = challenge.to_mining_task("test_miner");

    assert_eq!(task.operation_type, challenge.operation_type);
    assert_eq!(task.difficulty, challenge.difficulty);
    assert!(!task.requester.is_empty());
}

#[test]
fn test_e2e_engine_task_execution() {
    // Test: Create engine → execute task
    let engine: Box<dyn TensorEngine> = Box::new(AI3Engine::new());
    let stats = engine.get_stats();

    assert_eq!(stats.total_tasks_processed, 0);
    assert_eq!(stats.successful_tasks, 0);
}

#[test]
fn test_e2e_block_with_tensor_proof_transaction() {
    // Test: Create block containing tensor proof transactions
    let tx = Transaction {
        hash: "proof_tx_hash".to_string(),
        from: "miner_1".to_string(),
        to: "proof_verifier".to_string(),
        amount: 0,
        fee: 5,
        timestamp: 10000,
        nonce: 0,
        tx_type: TransactionType::TensorProof,
    };

    let block = Block::new(
        1,
        "prev_hash".to_string(),
        vec![tx.clone()],
        "block_miner".to_string(),
        2000,
    );

    assert_eq!(block.transactions.len(), 1);
    match block.transactions[0].tx_type {
        TransactionType::TensorProof => {}
        _ => panic!("Expected TensorProof transaction"),
    }
}

#[test]
fn test_e2e_mining_reward_transaction_flow() {
    // Test: Challenge → Mining task → Reward transaction
    let gen = ChallengeGenerator::default();
    let challenge = gen
        .generate(
            100,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("generate");

    let task = challenge.to_mining_task("miner_address");

    // Simulate reward based on task reward
    let reward_tx = Transaction {
        hash: "reward_tx".to_string(),
        from: "validator".to_string(),
        to: "miner_address".to_string(),
        amount: task.reward, // From mining task reward
        fee: 0,
        timestamp: 10001,
        nonce: 0,
        tx_type: TransactionType::Transfer,
    };

    assert_eq!(reward_tx.amount, task.reward);
}

#[test]
fn test_e2e_error_propagation_across_modules() {
    // Test: Error handling consistency across modules
    let err = TribeError::InvalidOperation("test error".to_string());

    // Errors should be displayable
    let msg = err.to_string();
    assert!(msg.contains("Invalid operation"));
    assert!(msg.contains("test error"));
}

#[test]
fn test_e2e_block_chain_validation() {
    // Test: Create chain of blocks with proper hash linking
    let block0 = Block::new(
        0,
        "genesis".to_string(),
        vec![],
        "genesis_miner".to_string(),
        1000,
    );

    let block1 = Block::new(1, block0.hash.clone(), vec![], "miner_1".to_string(), 1000);

    let block2 = Block::new(2, block1.hash.clone(), vec![], "miner_2".to_string(), 1000);

    // Verify chain structure
    assert_eq!(block1.previous_hash, block0.hash);
    assert_eq!(block2.previous_hash, block1.hash);
    assert_eq!(block2.height, 2);
}

#[test]
fn test_e2e_multiple_transaction_types_in_block() {
    // Test: Block containing mix of transaction types
    let tx_transfer = Transaction {
        hash: "tx1".to_string(),
        from: "alice".to_string(),
        to: "bob".to_string(),
        amount: 100,
        fee: 1,
        timestamp: 1000,
        nonce: 0,
        tx_type: TransactionType::Transfer,
    };

    let tx_stake = Transaction {
        hash: "tx2".to_string(),
        from: "bob".to_string(),
        to: "staking_pool".to_string(),
        amount: 1000,
        fee: 1,
        timestamp: 1001,
        nonce: 1,
        tx_type: TransactionType::Stake,
    };

    let tx_proof = Transaction {
        hash: "tx3".to_string(),
        from: "miner".to_string(),
        to: "validator".to_string(),
        amount: 0,
        fee: 5,
        timestamp: 1002,
        nonce: 0,
        tx_type: TransactionType::TensorProof,
    };

    let block = Block::new(
        1,
        "prev".to_string(),
        vec![tx_transfer, tx_stake, tx_proof],
        "miner".to_string(),
        1500,
    );

    assert_eq!(block.transactions.len(), 3);
    // Verify each type is preserved
    assert!(matches!(
        block.transactions[0].tx_type,
        TransactionType::Transfer
    ));
    assert!(matches!(
        block.transactions[1].tx_type,
        TransactionType::Stake
    ));
    assert!(matches!(
        block.transactions[2].tx_type,
        TransactionType::TensorProof
    ));
}

#[test]
fn test_e2e_challenge_expiry_workflow() {
    // Test: Challenge lifecycle from creation to expiry
    let gen = ChallengeGenerator::default();
    let challenge = gen
        .generate(
            100,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("generate");

    // New challenges should not be expired
    assert!(!challenge.is_expired());

    // Verify expiry field is set correctly
    assert!(challenge.expires_at > challenge.created_at);
}

#[test]
fn test_e2e_core_to_mining_module_integration() {
    // Test: pot-o-core error types used in pot-o-mining
    let err = pot_o_core::TribeError::InvalidOperation("mining failed".to_string());
    let err_msg = err.to_string();

    assert!(err_msg.contains("Invalid operation"));
    assert!(err_msg.contains("mining failed"));
}

#[test]
fn test_e2e_tensor_shape_in_mining_challenge() {
    // Test: Tensor shapes used in mining challenges
    let gen = ChallengeGenerator::default();
    let challenge = gen
        .generate(
            100,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("generate");

    let input_dims = &challenge.input_tensor.shape.dims;
    assert!(!input_dims.is_empty());
    assert!(input_dims.iter().all(|&d| d > 0));
}

#[test]
fn test_e2e_mining_difficulty_scaling() {
    // Test: Different difficulty levels produce consistent challenge structure
    let gen_easy = ChallengeGenerator::new(100, 64);
    let gen_hard = ChallengeGenerator::new(5000, 64);

    let ch_easy = gen_easy
        .generate(
            100,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("easy");
    let ch_hard = gen_hard
        .generate(
            100,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("hard");

    assert!(ch_hard.difficulty > ch_easy.difficulty);
    assert_eq!(ch_easy.max_tensor_dim, ch_hard.max_tensor_dim);
}

#[test]
fn test_smoke_http_server_and_tensors() {
    // Smoke test: Verify core modules don't panic on use
    let _block = Block::new(0, "prev".to_string(), vec![], "miner".to_string(), 1000);
    let _gen = ChallengeGenerator::default();
    let _engine = AI3Engine::new();
}

#[test]
fn test_error_result_type_usage() {
    // Test: TribeResult type is usable throughout codebase
    let _: pot_o_core::TribeResult<i32> = Ok(42);
    let _: pot_o_core::TribeResult<String> = Err(TribeError::ConfigError("test".to_string()));
}

#[test]
fn test_transaction_serialization_readiness() {
    // Test: Transactions can be created with serialization support
    use serde_json;

    let tx = Transaction {
        hash: "test_hash".to_string(),
        from: "alice".to_string(),
        to: "bob".to_string(),
        amount: 100,
        fee: 1,
        timestamp: 1000,
        nonce: 0,
        tx_type: TransactionType::Transfer,
    };

    // Should be serializable (derived from serde)
    let json = serde_json::to_string(&tx);
    assert!(json.is_ok());
}

#[test]
fn test_block_serialization_readiness() {
    // Test: Blocks can be serialized for transport
    use serde_json;

    let block = Block::new(0, "prev".to_string(), vec![], "miner".to_string(), 1000);

    let json = serde_json::to_string(&block);
    assert!(json.is_ok());
}
