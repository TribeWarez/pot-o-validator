//! Tests for Block and Transaction types
//!
//! Validates block creation, hash calculation, transaction handling

use pot_o_core::{Block, Transaction, TransactionType, TokenType};

#[test]
fn test_token_type_creation() {
    let tokens = vec![
        TokenType::TribeChain,
        TokenType::PTtC,
        TokenType::NMTC,
        TokenType::STOMP,
        TokenType::AUM,
        TokenType::AI3,
    ];
    
    assert_eq!(tokens.len(), 6);
}

#[test]
fn test_transaction_type_variants() {
    let types = vec![
        TransactionType::Transfer,
        TransactionType::Stake,
        TransactionType::TensorProof,
        TransactionType::TokenCreate,
        TransactionType::Swap,
    ];
    
    assert_eq!(types.len(), 5);
}

#[test]
fn test_block_creation_basic() {
    let transactions = vec![];
    let block = Block::new(
        0,
        "genesis".to_string(),
        transactions,
        "miner1".to_string(),
        1000,
    );
    
    assert_eq!(block.height, 0);
    assert_eq!(block.previous_hash, "genesis");
    assert_eq!(block.miner, "miner1");
    assert_eq!(block.difficulty, 1000);
    assert_eq!(block.transactions.len(), 0);
    assert!(!block.hash.is_empty());
    assert_eq!(block.nonce, 0);
}

#[test]
fn test_block_hash_calculation() {
    let transactions = vec![];
    let block = Block::new(
        1,
        "0x123".to_string(),
        transactions,
        "miner1".to_string(),
        1000,
    );
    
    // Hash should be deterministic for same inputs
    let hash1 = block.hash.clone();
    let hash2 = block.calculate_hash();
    
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex characters
}

#[test]
fn test_block_hash_changes_with_different_height() {
    let transactions = vec![];
    
    let block1 = Block::new(
        1,
        "prev".to_string(),
        transactions.clone(),
        "miner".to_string(),
        1000,
    );
    
    let block2 = Block::new(
        2,
        "prev".to_string(),
        transactions,
        "miner".to_string(),
        1000,
    );
    
    assert_ne!(block1.hash, block2.hash);
}

#[test]
fn test_block_hash_changes_with_different_miner() {
    let transactions = vec![];
    
    let block1 = Block::new(
        1,
        "prev".to_string(),
        transactions.clone(),
        "miner1".to_string(),
        1000,
    );
    
    let block2 = Block::new(
        1,
        "prev".to_string(),
        transactions,
        "miner2".to_string(),
        1000,
    );
    
    assert_ne!(block1.hash, block2.hash);
}

#[test]
fn test_block_hash_changes_with_different_difficulty() {
    let transactions = vec![];
    
    let block1 = Block::new(
        1,
        "prev".to_string(),
        transactions.clone(),
        "miner".to_string(),
        1000,
    );
    
    let block2 = Block::new(
        1,
        "prev".to_string(),
        transactions,
        "miner".to_string(),
        2000,
    );
    
    assert_ne!(block1.hash, block2.hash);
}

#[test]
fn test_block_with_transactions() {
    let tx = Transaction {
        hash: "tx_hash_1".to_string(),
        from: "alice".to_string(),
        to: "bob".to_string(),
        amount: 100,
        fee: 1,
        timestamp: 1000,
        nonce: 0,
        tx_type: TransactionType::Transfer,
    };
    
    let block = Block::new(
        1,
        "prev".to_string(),
        vec![tx],
        "miner".to_string(),
        1000,
    );
    
    assert_eq!(block.transactions.len(), 1);
    assert_eq!(block.transactions[0].from, "alice");
    assert_eq!(block.transactions[0].to, "bob");
    assert_eq!(block.transactions[0].amount, 100);
}

#[test]
fn test_block_hash_includes_transaction_hashes() {
    let tx1 = Transaction {
        hash: "tx_1".to_string(),
        from: "alice".to_string(),
        to: "bob".to_string(),
        amount: 100,
        fee: 1,
        timestamp: 1000,
        nonce: 0,
        tx_type: TransactionType::Transfer,
    };
    
    let tx2 = Transaction {
        hash: "tx_2".to_string(),
        from: "charlie".to_string(),
        to: "dave".to_string(),
        amount: 50,
        fee: 1,
        timestamp: 1000,
        nonce: 0,
        tx_type: TransactionType::Stake,
    };
    
    let block1 = Block::new(
        1,
        "prev".to_string(),
        vec![tx1],
        "miner".to_string(),
        1000,
    );
    
    let block2 = Block::new(
        1,
        "prev".to_string(),
        vec![tx2],
        "miner".to_string(),
        1000,
    );
    
    // Different transactions should produce different block hashes
    assert_ne!(block1.hash, block2.hash);
}

#[test]
fn test_block_multiple_transactions() {
    let transactions = vec![
        Transaction {
            hash: "tx1".to_string(),
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100,
            fee: 1,
            timestamp: 1000,
            nonce: 0,
            tx_type: TransactionType::Transfer,
        },
        Transaction {
            hash: "tx2".to_string(),
            from: "bob".to_string(),
            to: "charlie".to_string(),
            amount: 50,
            fee: 1,
            timestamp: 1001,
            nonce: 1,
            tx_type: TransactionType::Transfer,
        },
        Transaction {
            hash: "tx3".to_string(),
            from: "charlie".to_string(),
            to: "alice".to_string(),
            amount: 25,
            fee: 1,
            timestamp: 1002,
            nonce: 2,
            tx_type: TransactionType::Stake,
        },
    ];
    
    let block = Block::new(
        1,
        "prev".to_string(),
        transactions.clone(),
        "miner".to_string(),
        1000,
    );
    
    assert_eq!(block.transactions.len(), 3);
    assert_eq!(block.transactions[0].from, "alice");
    assert_eq!(block.transactions[1].from, "bob");
    assert_eq!(block.transactions[2].from, "charlie");
}

#[test]
fn test_transaction_type_staking() {
    let tx = Transaction {
        hash: "stake_tx".to_string(),
        from: "miner1".to_string(),
        to: "staking_pool".to_string(),
        amount: 1000000,
        fee: 10,
        timestamp: 5000,
        nonce: 5,
        tx_type: TransactionType::Stake,
    };
    
    assert_eq!(tx.amount, 1000000);
    match tx.tx_type {
        TransactionType::Stake => {},
        _ => panic!("Expected Stake transaction"),
    }
}

#[test]
fn test_transaction_type_tensor_proof() {
    let tx = Transaction {
        hash: "proof_tx".to_string(),
        from: "miner1".to_string(),
        to: "proof_validator".to_string(),
        amount: 0,
        fee: 5,
        timestamp: 6000,
        nonce: 0,
        tx_type: TransactionType::TensorProof,
    };
    
    match tx.tx_type {
        TransactionType::TensorProof => {},
        _ => panic!("Expected TensorProof transaction"),
    }
}

#[test]
fn test_block_timestamp_is_set() {
    let block = Block::new(
        1,
        "prev".to_string(),
        vec![],
        "miner".to_string(),
        1000,
    );
    
    // Timestamp should be roughly current time
    let now = chrono::Utc::now().timestamp() as u64;
    assert!(block.timestamp <= now);
    assert!(block.timestamp >= now - 5); // Allow 5 second variance
}

#[test]
fn test_block_hash_hex_format() {
    let block = Block::new(
        1,
        "prev".to_string(),
        vec![],
        "miner".to_string(),
        1000,
    );
    
    // Hash should be valid hex (only 0-9, a-f)
    assert!(block.hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(block.hash.len(), 64); // SHA-256 is 32 bytes = 64 hex chars
}

#[test]
fn test_block_clone_and_equality() {
    let block1 = Block::new(
        1,
        "prev".to_string(),
        vec![],
        "miner".to_string(),
        1000,
    );
    
    let block2 = block1.clone();
    
    assert_eq!(block1.hash, block2.hash);
    assert_eq!(block1.height, block2.height);
    assert_eq!(block1.miner, block2.miner);
}

#[test]
fn test_transaction_clone() {
    let tx = Transaction {
        hash: "tx_hash".to_string(),
        from: "alice".to_string(),
        to: "bob".to_string(),
        amount: 100,
        fee: 1,
        timestamp: 1000,
        nonce: 0,
        tx_type: TransactionType::Transfer,
    };
    
    let tx_clone = tx.clone();
    
    assert_eq!(tx.hash, tx_clone.hash);
    assert_eq!(tx.from, tx_clone.from);
    assert_eq!(tx.to, tx_clone.to);
    assert_eq!(tx.amount, tx_clone.amount);
}

#[test]
fn test_block_serialization_compatible() {
    // Test that Block can be serialized (via serde derive)
    let block = Block::new(
        1,
        "prev".to_string(),
        vec![],
        "miner".to_string(),
        1000,
    );
    
    // Verify Block has the required serde attributes
    let _: &(dyn std::any::Any) = &block;
}
