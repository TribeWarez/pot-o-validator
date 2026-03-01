use async_trait::async_trait;
use borsh::BorshSerialize;
use pot_o_core::{TribeError, TribeResult};
use pot_o_mining::ProofPayload;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer as SolSigner},
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSignature(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerAccount {
    pub pubkey: String,
    pub total_proofs: u64,
    pub total_rewards: u64,
    pub pending_rewards: u64,
    pub reputation_score: f64,
    pub last_proof_slot: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Token {
    SOL,
    PTtC,
    NMTC,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// How proofs and rewards interact with on-chain programs.
#[async_trait]
pub trait ChainBridge: Send + Sync {
    async fn submit_proof(&self, proof: &ProofPayload) -> TribeResult<TxSignature>;
    async fn query_miner(&self, pubkey: &str) -> TribeResult<Option<MinerAccount>>;
    async fn get_current_difficulty(&self) -> TribeResult<u64>;
    async fn request_swap(
        &self,
        from_token: Token,
        to_token: Token,
        amount: u64,
    ) -> TribeResult<TxSignature>;
}

// ---------------------------------------------------------------------------
// On-chain ProofParams mirror (Borsh-serialized for Anchor IX)
// ---------------------------------------------------------------------------

#[derive(BorshSerialize)]
struct OnChainProofParams {
    challenge_id: [u8; 32],
    challenge_slot: u64,
    tensor_result_hash: [u8; 32],
    mml_score: u64,
    path_signature: [u8; 32],
    path_distance: u32,
    computation_nonce: u64,
    computation_hash: [u8; 32],
}

const MML_SCALE: f64 = 1_000_000_000.0;

fn hex_to_32(hex_str: &str) -> Result<[u8; 32], TribeError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| TribeError::ChainBridgeError(format!("hex decode: {e}")))?;
    bytes
        .try_into()
        .map_err(|_| TribeError::ChainBridgeError("expected 32 bytes from hex".into()))
}

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}"));
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

// ---------------------------------------------------------------------------
// SolanaBridge
// ---------------------------------------------------------------------------

pub struct SolanaBridge {
    pub rpc_url: String,
    pub program_id: Pubkey,
    relayer_keypair: Option<Arc<Keypair>>,
}

impl SolanaBridge {
    pub fn new(rpc_url: String, program_id: String, keypair_path: String) -> Self {
        let pid = Pubkey::from_str(&program_id).unwrap_or_else(|e| {
            tracing::warn!(error = %e, program_id, "Invalid program_id, using default");
            Pubkey::default()
        });

        let kp = match read_keypair_file(&keypair_path) {
            Ok(k) => {
                tracing::info!(
                    relayer = %k.pubkey(),
                    path = %keypair_path,
                    "Loaded relayer keypair"
                );
                Some(Arc::new(k))
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %keypair_path,
                    "Relayer keypair not found; on-chain submissions will return stub signatures"
                );
                None
            }
        };

        Self {
            rpc_url,
            program_id: pid,
            relayer_keypair: kp,
        }
    }

    fn build_submit_proof_ix(
        &self,
        proof: &ProofPayload,
        challenge_slot: u64,
    ) -> TribeResult<Instruction> {
        let miner_pubkey = Pubkey::from_str(&proof.proof.miner_pubkey)
            .map_err(|e| TribeError::ChainBridgeError(format!("invalid miner pubkey: {e}")))?;

        let relayer_pubkey = self
            .relayer_keypair
            .as_ref()
            .expect("checked before calling")
            .pubkey();

        let (config_pda, _) =
            Pubkey::find_program_address(&[b"pot_o_config"], &self.program_id);
        let (miner_pda, _) =
            Pubkey::find_program_address(&[b"miner", miner_pubkey.as_ref()], &self.program_id);

        let challenge_id_bytes = hex_to_32(&proof.proof.challenge_id)?;
        let (proof_pda, _) =
            Pubkey::find_program_address(&[b"proof", challenge_id_bytes.as_ref()], &self.program_id);

        let params = OnChainProofParams {
            challenge_id: challenge_id_bytes,
            challenge_slot,
            tensor_result_hash: hex_to_32(&proof.proof.tensor_result_hash)?,
            mml_score: (proof.proof.mml_score * MML_SCALE) as u64,
            path_signature: hex_to_32(&proof.proof.path_signature)?,
            path_distance: proof.proof.path_distance,
            computation_nonce: proof.proof.computation_nonce,
            computation_hash: hex_to_32(&proof.proof.computation_hash)?,
        };

        let disc = anchor_discriminator("submit_proof");
        let params_data = borsh::to_vec(&params)
            .map_err(|e| TribeError::ChainBridgeError(format!("borsh serialize: {e}")))?;
        let mut data = Vec::with_capacity(8 + params_data.len());
        data.extend_from_slice(&disc);
        data.extend_from_slice(&params_data);

        let accounts = vec![
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(miner_pubkey, false),
            AccountMeta::new(miner_pda, false),
            AccountMeta::new(proof_pda, false),
            AccountMeta::new(relayer_pubkey, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data,
        })
    }

    fn stub_signature(proof: &ProofPayload) -> TxSignature {
        let hash = &proof.proof.computation_hash;
        let suffix = if hash.len() >= 16 { &hash[..16] } else { hash };
        TxSignature(format!("sim_tx_{suffix}"))
    }
}

#[async_trait]
impl ChainBridge for SolanaBridge {
    async fn submit_proof(&self, proof: &ProofPayload) -> TribeResult<TxSignature> {
        let kp = match &self.relayer_keypair {
            Some(k) => Arc::clone(k),
            None => {
                tracing::warn!(
                    challenge = %proof.proof.challenge_id,
                    miner = %proof.proof.miner_pubkey,
                    "No relayer keypair; returning stub signature"
                );
                return Ok(Self::stub_signature(proof));
            }
        };

        tracing::info!(
            challenge = %proof.proof.challenge_id,
            miner = %proof.proof.miner_pubkey,
            program = %self.program_id,
            relayer = %kp.pubkey(),
            "Submitting proof to Solana"
        );

        let rpc_url = self.rpc_url.clone();
        let proof_clone = proof.clone();

        let rpc_url_slot = rpc_url.clone();
        let challenge_slot = tokio::task::spawn_blocking(move || {
            let client = RpcClient::new(&rpc_url_slot);
            client.get_slot()
        })
        .await
        .map_err(|e| TribeError::ChainBridgeError(format!("spawn_blocking join: {e}")))?
        .map_err(|e| TribeError::ChainBridgeError(format!("get_slot: {e}")))?;

        tracing::debug!(challenge_slot, "Fetched current Solana slot");

        let ix = self.build_submit_proof_ix(&proof_clone, challenge_slot)?;

        let sig = tokio::task::spawn_blocking(move || -> TribeResult<String> {
            let client = RpcClient::new(&rpc_url);
            let blockhash = client
                .get_latest_blockhash()
                .map_err(|e| TribeError::ChainBridgeError(format!("get_latest_blockhash: {e}")))?;

            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&kp.pubkey()),
                &[&kp],
                blockhash,
            );

            let signature = client
                .send_and_confirm_transaction(&tx)
                .map_err(|e| TribeError::ChainBridgeError(format!("send_and_confirm: {e}")))?;

            Ok(signature.to_string())
        })
        .await
        .map_err(|e| TribeError::ChainBridgeError(format!("spawn_blocking join: {e}")))??;

        tracing::info!(tx_signature = %sig, "Proof submitted to Solana successfully");
        Ok(TxSignature(sig))
    }

    async fn query_miner(&self, pubkey: &str) -> TribeResult<Option<MinerAccount>> {
        tracing::debug!(pubkey, "Querying miner account on-chain");
        Ok(None)
    }

    async fn get_current_difficulty(&self) -> TribeResult<u64> {
        Ok(2)
    }

    async fn request_swap(
        &self,
        from_token: Token,
        to_token: Token,
        amount: u64,
    ) -> TribeResult<TxSignature> {
        tracing::info!(?from_token, ?to_token, amount, "Swap request (CPI to tribewarez-swap)");
        Ok(TxSignature("sim_swap_placeholder".into()))
    }
}

// ---------------------------------------------------------------------------
// EvmBridge (stubbed)
// ---------------------------------------------------------------------------

pub struct EvmBridge {
    pub rpc_url: String,
    pub contract_address: String,
}

#[async_trait]
impl ChainBridge for EvmBridge {
    async fn submit_proof(&self, _proof: &ProofPayload) -> TribeResult<TxSignature> {
        todo!("EVM proof submission not yet implemented")
    }
    async fn query_miner(&self, _pubkey: &str) -> TribeResult<Option<MinerAccount>> {
        todo!("EVM miner query not yet implemented")
    }
    async fn get_current_difficulty(&self) -> TribeResult<u64> {
        todo!("EVM difficulty query not yet implemented")
    }
    async fn request_swap(&self, _from: Token, _to: Token, _amount: u64) -> TribeResult<TxSignature> {
        todo!("EVM swap not yet implemented")
    }
}

// ---------------------------------------------------------------------------
// CrossChainBridge (stubbed)
// ---------------------------------------------------------------------------

pub struct CrossChainBridge {
    pub solana_rpc_url: String,
    pub evm_rpc_url: String,
}

#[async_trait]
impl ChainBridge for CrossChainBridge {
    async fn submit_proof(&self, _proof: &ProofPayload) -> TribeResult<TxSignature> {
        todo!("Cross-chain proof submission not yet implemented")
    }
    async fn query_miner(&self, _pubkey: &str) -> TribeResult<Option<MinerAccount>> {
        todo!("Cross-chain miner query not yet implemented")
    }
    async fn get_current_difficulty(&self) -> TribeResult<u64> {
        todo!("Cross-chain difficulty query not yet implemented")
    }
    async fn request_swap(&self, _from: Token, _to: Token, _amount: u64) -> TribeResult<TxSignature> {
        todo!("Cross-chain atomic swap not yet implemented")
    }
}
