use std::fs;
use std::path::Path;

use pot_o_core::TokenType;
use pot_o_extensions::genesis::{Genesis, GenesisEntry};
use pot_o_extensions::load_ledger;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} --input ledger.json --output genesis.json [--chain-id <id>] [--faucet-address <addr> --faucet-amount <amt>]",
            args[0]
        );
        std::process::exit(1);
    }

    let mut input_path = String::new();
    let mut output_path = String::new();
    let mut chain_id = format!("tribechain-{}", chrono::Utc::now().timestamp());
    let mut faucet_address = String::new();
    let mut faucet_amount: u64 = 0;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                input_path = args[i].clone();
            }
            "--output" => {
                i += 1;
                output_path = args[i].clone();
            }
            "--chain-id" => {
                i += 1;
                chain_id = args[i].clone();
            }
            "--faucet-address" => {
                i += 1;
                faucet_address = args[i].clone();
            }
            "--faucet-amount" => {
                i += 1;
                faucet_amount = args[i].parse().expect("Invalid faucet amount");
            }
            _ => {
                eprintln!("Unknown arg: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if input_path.is_empty() || output_path.is_empty() {
        eprintln!("--input and --output are required");
        std::process::exit(1);
    }

    if !Path::new(&input_path).exists() {
        eprintln!("Input ledger file not found: {}", input_path);
        std::process::exit(1);
    }

    let protocol_fee_address = "genesis";
    let ledger = load_ledger(&input_path, protocol_fee_address);
    let mut entries = Vec::new();

    for ((address, token), balance) in ledger.balances() {
        if *balance > 0 {
            entries.push(GenesisEntry {
                address: address.clone(),
                token: token.clone(),
                balance: *balance,
            });
        }
    }

    if !faucet_address.is_empty() && faucet_amount > 0 {
        entries.push(GenesisEntry {
            address: faucet_address,
            token: TokenType::TribeChain,
            balance: faucet_amount,
        });
    }

    let genesis = Genesis {
        entries,
        chain_id,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        tribechain_genesis_version: 1,
    };

    if let Err(e) = genesis.validate() {
        eprintln!("Genesis validation failed: {}", e);
        std::process::exit(1);
    }

    let json = serde_json::to_string_pretty(&genesis).expect("Failed to serialize genesis");
    fs::write(&output_path, &json).expect("Failed to write genesis file");
    println!(
        "Genesis written to {} ({} entries)",
        output_path,
        genesis.entries.len()
    );
}
