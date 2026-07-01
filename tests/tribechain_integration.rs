//! Tribechain integration tests
//!
//! Run: cargo test --test tribechain_integration 2>&1 | tail -30
//!
//! These tests verify Tribechain token ledger functionality end-to-end.

use pot_o_core::TokenType;
use pot_o_extensions::ledger::Ledger;
use pot_o_extensions::tx::TransferTransaction;

#[test]
fn test_transfer_and_fee_payment() {
    let mut ledger = Ledger::new("protocol".to_string());
    ledger.issue("alice", &TokenType::TribeChain, 1000);
    ledger.issue("bob", &TokenType::TribeChain, 500);

    let alice_initial = ledger.balance_of("alice", &TokenType::TribeChain);
    let bob_initial = ledger.balance_of("bob", &TokenType::TribeChain);
    let miner_initial = ledger.balance_of("miner", &TokenType::TribeChain);

    let tx = TransferTransaction {
        tx_hash: [0u8; 32],
        nonce: 0,
        from: "alice".to_string(),
        to: "bob".to_string(),
        token: TokenType::TribeChain,
        amount: 200,
        fee: 5,
        signature: vec![],
        timestamp: 0,
    };

    let receipt = ledger.apply_transfer(&tx, "miner").unwrap();
    assert_eq!(receipt.amount, 200);
    assert_eq!(receipt.fee, 5);
    assert_eq!(
        ledger.balance_of("alice", &TokenType::TribeChain),
        alice_initial - 205
    );
    assert_eq!(
        ledger.balance_of("bob", &TokenType::TribeChain),
        bob_initial + 200
    );
    assert_eq!(
        ledger.balance_of("miner", &TokenType::TribeChain),
        miner_initial + 5
    );
    assert_eq!(ledger.current_nonce("alice"), 1);
}

#[test]
fn test_nonce_tracking_on_multiple_transfers() {
    let mut ledger = Ledger::new("protocol".to_string());
    ledger.issue("alice", &TokenType::TribeChain, 5000);

    let tx1 = TransferTransaction {
        tx_hash: [1u8; 32],
        nonce: 0,
        from: "alice".to_string(),
        to: "bob".to_string(),
        token: TokenType::TribeChain,
        amount: 100,
        fee: 1,
        signature: vec![],
        timestamp: 0,
    };
    ledger.apply_transfer(&tx1, "miner").unwrap();
    assert_eq!(ledger.current_nonce("alice"), 1);

    let tx2 = TransferTransaction {
        tx_hash: [2u8; 32],
        nonce: 1,
        from: "alice".to_string(),
        to: "carol".to_string(),
        token: TokenType::TribeChain,
        amount: 200,
        fee: 2,
        signature: vec![],
        timestamp: 1,
    };
    ledger.apply_transfer(&tx2, "miner").unwrap();
    assert_eq!(ledger.current_nonce("alice"), 2);

    let tx3 = TransferTransaction {
        tx_hash: [3u8; 32],
        nonce: 2,
        from: "alice".to_string(),
        to: "dave".to_string(),
        token: TokenType::TribeChain,
        amount: 300,
        fee: 3,
        signature: vec![],
        timestamp: 2,
    };
    ledger.apply_transfer(&tx3, "miner").unwrap();
    assert_eq!(ledger.current_nonce("alice"), 3);
}

#[test]
fn test_coinbase_maturity_enforced() {
    let ledger = Ledger::new("protocol".to_string());

    assert!(!ledger.is_coinbase_mature("miner", 0, 50));
    assert!(ledger.is_coinbase_mature("miner", 0, 100));
    assert!(ledger.is_coinbase_mature("miner", 0, 200));
}

#[test]
fn test_multi_token_independent_balances() {
    let mut ledger = Ledger::new("protocol".to_string());

    ledger.issue("alice", &TokenType::TribeChain, 1000);
    ledger.issue("alice", &TokenType::PTtC, 500);
    ledger.issue("alice", &TokenType::NMTC, 250);

    assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 1000);
    assert_eq!(ledger.balance_of("alice", &TokenType::PTtC), 500);
    assert_eq!(ledger.balance_of("alice", &TokenType::NMTC), 250);
    assert_eq!(ledger.balance_of("alice", &TokenType::STOMP), 0);

    let tx = TransferTransaction {
        tx_hash: [0u8; 32],
        nonce: 0,
        from: "alice".to_string(),
        to: "bob".to_string(),
        token: TokenType::PTtC,
        amount: 100,
        fee: 0,
        signature: vec![],
        timestamp: 0,
    };
    ledger.apply_transfer(&tx, "miner").unwrap();

    assert_eq!(ledger.balance_of("alice", &TokenType::PTtC), 400);
    assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 1000);
}

#[test]
fn test_total_supply_tracking() {
    let mut ledger = Ledger::new("protocol".to_string());
    assert_eq!(ledger.total_supply_of(&TokenType::TribeChain), 0);

    ledger
        .mint_tokens("alice", &TokenType::TribeChain, 1000)
        .unwrap();
    assert_eq!(ledger.total_supply_of(&TokenType::TribeChain), 1000);

    ledger
        .mint_tokens("bob", &TokenType::TribeChain, 500)
        .unwrap();
    assert_eq!(ledger.total_supply_of(&TokenType::TribeChain), 1500);
}

#[test]
fn test_tx_history_recorded() {
    let mut ledger = Ledger::new("protocol".to_string());
    ledger.issue("alice", &TokenType::TribeChain, 1000);

    let tx = TransferTransaction {
        tx_hash: [1u8; 32],
        nonce: 0,
        from: "alice".to_string(),
        to: "bob".to_string(),
        token: TokenType::TribeChain,
        amount: 300,
        fee: 10,
        signature: vec![],
        timestamp: 100,
    };
    ledger.apply_transfer(&tx, "miner").unwrap();

    let history = ledger.tx_history_for("alice");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].amount, 300);
    assert_eq!(history[0].from, "alice");
    assert_eq!(history[0].to, "bob");

    let bob_history = ledger.tx_history_for("bob");
    assert_eq!(bob_history.len(), 1);
}
