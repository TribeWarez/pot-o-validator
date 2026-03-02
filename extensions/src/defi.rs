//! HTTP API support for tribewarez-staking, tribewarez-swap, tribewarez-vault.
//! Fetches on-chain account data via Solana RPC and returns JSON-serializable types.

use pot_o_core::TribeResult;
use serde::Serialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// Program IDs (from declare_id! in each program)
pub const STAKING_PROGRAM_ID: &str = "Go2BZRhNLoaVni3QunrKPAXYdHtwZtTXuVspxpdAeDS8";
pub const SWAP_PROGRAM_ID: &str = "GPGGnKwnvKseSxzPukrNvch1CwYhifTqgj2RdW1P26H3";
pub const VAULT_PROGRAM_ID: &str = "HmWGA3JAF6basxGCvvGNHAdTBE3qCPhJCeFJAd7r5ra9";

const ANCHOR_DISCRIMINATOR_LEN: usize = 8;

fn pubkey_to_string(p: &Pubkey) -> String {
    p.to_string()
}

// ---------------------------------------------------------------------------
// Staking (mirror account layouts for Borsh deserialize after 8-byte discriminator)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StakingPoolInfo {
    pub pubkey: String,
    pub authority: String,
    pub token_mint: String,
    pub reward_mint: String,
    pub pool_token_account: String,
    pub reward_token_account: String,
    pub reward_rate: u64,
    pub lock_duration: i64,
    pub total_staked: u64,
    pub total_rewards_distributed: u64,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StakeAccountInfo {
    pub pubkey: String,
    pub owner: String,
    pub pool: String,
    pub amount: u64,
    pub stake_time: i64,
    pub unlock_time: i64,
    pub last_reward_time: i64,
    pub pending_rewards: u64,
    pub total_rewards_claimed: u64,
}

fn parse_staking_pool(pubkey: &Pubkey, data: &[u8]) -> TribeResult<StakingPoolInfo> {
    let data = data
        .get(ANCHOR_DISCRIMINATOR_LEN..)
        .ok_or_else(|| pot_o_core::TribeError::ChainBridgeError("account data too short".into()))?;
    if data.len() < 32 * 5 + 8 * 5 + 2 {
        return Err(pot_o_core::TribeError::ChainBridgeError(
            "staking pool account data too short".into(),
        ));
    }
    let mut off = 0;
    let read_pubkey = |off: &mut usize| {
        let slice: [u8; 32] = data[*off..*off + 32].try_into().unwrap();
        *off += 32;
        Pubkey::new_from_array(slice)
    };
    let authority = read_pubkey(&mut off);
    let token_mint = read_pubkey(&mut off);
    let reward_mint = read_pubkey(&mut off);
    let pool_token_account = read_pubkey(&mut off);
    let reward_token_account = read_pubkey(&mut off);
    let reward_rate = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let lock_duration = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let total_staked = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let total_rewards_distributed = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let bump = data[off];
    off += 1;
    let _ = bump;
    let is_active = data[off] != 0;
    off += 1;
    let created_at = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    Ok(StakingPoolInfo {
        pubkey: pubkey_to_string(pubkey),
        authority: pubkey_to_string(&authority),
        token_mint: pubkey_to_string(&token_mint),
        reward_mint: pubkey_to_string(&reward_mint),
        pool_token_account: pubkey_to_string(&pool_token_account),
        reward_token_account: pubkey_to_string(&reward_token_account),
        reward_rate,
        lock_duration,
        total_staked,
        total_rewards_distributed,
        is_active,
        created_at,
    })
}

fn parse_stake_account(pubkey: &Pubkey, data: &[u8]) -> TribeResult<StakeAccountInfo> {
    let data = data
        .get(ANCHOR_DISCRIMINATOR_LEN..)
        .ok_or_else(|| pot_o_core::TribeError::ChainBridgeError("account data too short".into()))?;
    if data.len() < 32 * 2 + 8 * 6 {
        return Err(pot_o_core::TribeError::ChainBridgeError(
            "stake account data too short".into(),
        ));
    }
    let mut off = 0;
    let read_pubkey = |off: &mut usize| {
        let slice: [u8; 32] = data[*off..*off + 32].try_into().unwrap();
        *off += 32;
        Pubkey::new_from_array(slice)
    };
    let owner = read_pubkey(&mut off);
    let pool = read_pubkey(&mut off);
    let amount = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let stake_time = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let unlock_time = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let last_reward_time = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let pending_rewards = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let total_rewards_claimed = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    Ok(StakeAccountInfo {
        pubkey: pubkey_to_string(pubkey),
        owner: pubkey_to_string(&owner),
        pool: pubkey_to_string(&pool),
        amount,
        stake_time,
        unlock_time,
        last_reward_time,
        pending_rewards,
        total_rewards_claimed,
    })
}

// ---------------------------------------------------------------------------
// Swap
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LiquidityPoolInfo {
    pub pubkey: String,
    pub authority: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_vault: String,
    pub token_b_vault: String,
    pub lp_mint: String,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub total_lp_supply: u64,
    pub swap_fee_bps: u64,
    pub protocol_fee_bps: u64,
    pub collected_fees_a: u64,
    pub collected_fees_b: u64,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapQuoteInfo {
    pub pool: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub price_impact_bps: u64,
}

fn parse_liquidity_pool(pubkey: &Pubkey, data: &[u8]) -> TribeResult<LiquidityPoolInfo> {
    let data = data
        .get(ANCHOR_DISCRIMINATOR_LEN..)
        .ok_or_else(|| pot_o_core::TribeError::ChainBridgeError("account data too short".into()))?;
    if data.len() < 32 * 6 + 8 * 6 + 2 + 8 {
        return Err(pot_o_core::TribeError::ChainBridgeError(
            "liquidity pool account data too short".into(),
        ));
    }
    let mut off = 0;
    let read_pubkey = |off: &mut usize| {
        let slice: [u8; 32] = data[*off..*off + 32].try_into().unwrap();
        *off += 32;
        Pubkey::new_from_array(slice)
    };
    let authority = read_pubkey(&mut off);
    let token_a_mint = read_pubkey(&mut off);
    let token_b_mint = read_pubkey(&mut off);
    let token_a_vault = read_pubkey(&mut off);
    let token_b_vault = read_pubkey(&mut off);
    let lp_mint = read_pubkey(&mut off);
    let reserve_a = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let reserve_b = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let total_lp_supply = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let swap_fee_bps = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let protocol_fee_bps = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let collected_fees_a = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let collected_fees_b = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let bump = data[off];
    off += 1;
    let _ = bump;
    let is_active = data[off] != 0;
    off += 1;
    let created_at = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    Ok(LiquidityPoolInfo {
        pubkey: pubkey_to_string(pubkey),
        authority: pubkey_to_string(&authority),
        token_a_mint: pubkey_to_string(&token_a_mint),
        token_b_mint: pubkey_to_string(&token_b_mint),
        token_a_vault: pubkey_to_string(&token_a_vault),
        token_b_vault: pubkey_to_string(&token_b_vault),
        lp_mint: pubkey_to_string(&lp_mint),
        reserve_a,
        reserve_b,
        total_lp_supply,
        swap_fee_bps,
        protocol_fee_bps,
        collected_fees_a,
        collected_fees_b,
        is_active,
        created_at,
    })
}

/// Constant product AMM: amount_out = (amount_in * (10000 - fee_bps) * reserve_out) / (reserve_in * 10000 + amount_in * (10000 - fee_bps))
fn calc_swap_output(amount_in: u64, reserve_in: u64, reserve_out: u64, fee_bps: u64) -> u64 {
    if reserve_in == 0 {
        return 0;
    }
    let amount_in_with_fee = (amount_in as u128) * ((10000 - fee_bps) as u128);
    let numerator = amount_in_with_fee * (reserve_out as u128);
    let denominator = (reserve_in as u128) * 10000 + amount_in_with_fee;
    (numerator / denominator) as u64
}

fn calc_fee(amount: u64, fee_bps: u64) -> u64 {
    ((amount as u128) * (fee_bps as u128) / 10000) as u64
}

fn calc_price_impact_bps(amount_in: u64, reserve_in: u64) -> u64 {
    if reserve_in == 0 {
        return 0;
    }
    ((amount_in as u128) * 10000 / (reserve_in as u128)) as u64
}

// ---------------------------------------------------------------------------
// Vault
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct TreasuryInfo {
    pub pubkey: String,
    pub authority: String,
    pub token_mint: String,
    pub vault_token_account: String,
    pub total_deposited: u64,
    pub total_vaults: u64,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserVaultInfo {
    pub pubkey: String,
    pub owner: String,
    pub treasury: String,
    pub name: String,
    pub balance: u64,
    pub lock_until: i64,
    pub created_at: i64,
    pub last_activity: i64,
    pub is_locked: bool,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EscrowInfo {
    pub pubkey: String,
    pub depositor: String,
    pub beneficiary: String,
    pub token_mint: String,
    pub escrow_token_account: String,
    pub amount: u64,
    pub release_time: i64,
    pub created_at: i64,
    pub is_released: bool,
    pub is_cancelled: bool,
}

fn parse_treasury(pubkey: &Pubkey, data: &[u8]) -> TribeResult<TreasuryInfo> {
    let data = data
        .get(ANCHOR_DISCRIMINATOR_LEN..)
        .ok_or_else(|| pot_o_core::TribeError::ChainBridgeError("account data too short".into()))?;
    if data.len() < 32 * 3 + 8 * 2 + 1 + 1 + 8 {
        return Err(pot_o_core::TribeError::ChainBridgeError(
            "treasury account data too short".into(),
        ));
    }
    let mut off = 0;
    let read_pubkey = |off: &mut usize| {
        let slice: [u8; 32] = data[*off..*off + 32].try_into().unwrap();
        *off += 32;
        Pubkey::new_from_array(slice)
    };
    let authority = read_pubkey(&mut off);
    let token_mint = read_pubkey(&mut off);
    let vault_token_account = read_pubkey(&mut off);
    let total_deposited = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let total_vaults = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let _bump = data[off];
    off += 1;
    let is_active = data[off] != 0;
    off += 1;
    let created_at = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    Ok(TreasuryInfo {
        pubkey: pubkey_to_string(pubkey),
        authority: pubkey_to_string(&authority),
        token_mint: pubkey_to_string(&token_mint),
        vault_token_account: pubkey_to_string(&vault_token_account),
        total_deposited,
        total_vaults,
        is_active,
        created_at,
    })
}

fn parse_user_vault(pubkey: &Pubkey, data: &[u8]) -> TribeResult<UserVaultInfo> {
    let data = data
        .get(ANCHOR_DISCRIMINATOR_LEN..)
        .ok_or_else(|| pot_o_core::TribeError::ChainBridgeError("account data too short".into()))?;
    // owner(32) + treasury(32) + name(4+bytes) + balance(8) + lock_until(8) + created_at(8) + last_activity(8) + is_locked(1) + total_deposited(8) + total_withdrawn(8)
    if data.len() < 32 + 32 + 4 + 8 + 8 + 8 + 8 + 1 + 8 + 8 {
        return Err(pot_o_core::TribeError::ChainBridgeError(
            "user vault account data too short".into(),
        ));
    }
    let mut off = 0;
    let read_pubkey = |off: &mut usize| {
        let slice: [u8; 32] = data[*off..*off + 32].try_into().unwrap();
        *off += 32;
        Pubkey::new_from_array(slice)
    };
    let owner = read_pubkey(&mut off);
    let treasury = read_pubkey(&mut off);
    let name_len = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as usize;
    off += 4;
    let name_len = name_len.min(32).min(data.len().saturating_sub(off));
    let name = String::from_utf8_lossy(&data[off..off + name_len]).to_string();
    off += name_len;
    let balance = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let lock_until = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let created_at = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let last_activity = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let is_locked = data[off] != 0;
    off += 1;
    let total_deposited = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let total_withdrawn = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    Ok(UserVaultInfo {
        pubkey: pubkey_to_string(pubkey),
        owner: pubkey_to_string(&owner),
        treasury: pubkey_to_string(&treasury),
        name,
        balance,
        lock_until,
        created_at,
        last_activity,
        is_locked,
        total_deposited,
        total_withdrawn,
    })
}

fn parse_escrow(pubkey: &Pubkey, data: &[u8]) -> TribeResult<EscrowInfo> {
    let data = data
        .get(ANCHOR_DISCRIMINATOR_LEN..)
        .ok_or_else(|| pot_o_core::TribeError::ChainBridgeError("account data too short".into()))?;
    if data.len() < 32 * 4 + 8 * 3 + 1 + 1 + 1 {
        return Err(pot_o_core::TribeError::ChainBridgeError(
            "escrow account data too short".into(),
        ));
    }
    let mut off = 0;
    let read_pubkey = |off: &mut usize| {
        let slice: [u8; 32] = data[*off..*off + 32].try_into().unwrap();
        *off += 32;
        Pubkey::new_from_array(slice)
    };
    let depositor = read_pubkey(&mut off);
    let beneficiary = read_pubkey(&mut off);
    let token_mint = read_pubkey(&mut off);
    let escrow_token_account = read_pubkey(&mut off);
    let amount = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let release_time = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let created_at = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let is_released = data[off] != 0;
    off += 1;
    let is_cancelled = data[off] != 0;
    off += 1;
    let _bump = data[off];
    Ok(EscrowInfo {
        pubkey: pubkey_to_string(pubkey),
        depositor: pubkey_to_string(&depositor),
        beneficiary: pubkey_to_string(&beneficiary),
        token_mint: pubkey_to_string(&token_mint),
        escrow_token_account: pubkey_to_string(&escrow_token_account),
        amount,
        release_time,
        created_at,
        is_released,
        is_cancelled,
    })
}

// ---------------------------------------------------------------------------
// DefiClient - RPC fetches
// ---------------------------------------------------------------------------

pub struct DefiClient {
    rpc_url: String,
    staking_program_id: Pubkey,
    swap_program_id: Pubkey,
    vault_program_id: Pubkey,
}

impl DefiClient {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url: rpc_url.clone(),
            staking_program_id: Pubkey::from_str(STAKING_PROGRAM_ID).unwrap(),
            swap_program_id: Pubkey::from_str(SWAP_PROGRAM_ID).unwrap(),
            vault_program_id: Pubkey::from_str(VAULT_PROGRAM_ID).unwrap(),
        }
    }

    fn get_account(&self, pubkey: &Pubkey) -> TribeResult<Vec<u8>> {
        let client = RpcClient::new(&self.rpc_url);
        let account = client.get_account(pubkey).map_err(|e| {
            pot_o_core::TribeError::ChainBridgeError(format!("rpc get_account: {e}"))
        })?;
        Ok(account.data)
    }

    // --- Staking ---
    pub fn get_staking_pool(&self, token_mint: &str) -> TribeResult<Option<StakingPoolInfo>> {
        let mint = Pubkey::from_str(token_mint)
            .map_err(|e| pot_o_core::TribeError::ChainBridgeError(format!("invalid mint: {e}")))?;
        let (pda, _) = Pubkey::find_program_address(
            &[b"staking_pool", mint.as_ref()],
            &self.staking_program_id,
        );
        let data = match self.get_account(&pda) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        parse_staking_pool(&pda, &data).map(Some)
    }

    pub fn get_stake_account(
        &self,
        pool_pubkey: &str,
        user_pubkey: &str,
    ) -> TribeResult<Option<StakeAccountInfo>> {
        let pool = Pubkey::from_str(pool_pubkey)
            .map_err(|e| pot_o_core::TribeError::ChainBridgeError(format!("invalid pool: {e}")))?;
        let user = Pubkey::from_str(user_pubkey)
            .map_err(|e| pot_o_core::TribeError::ChainBridgeError(format!("invalid user: {e}")))?;
        let (pda, _) = Pubkey::find_program_address(
            &[b"stake", pool.as_ref(), user.as_ref()],
            &self.staking_program_id,
        );
        let data = match self.get_account(&pda) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        parse_stake_account(&pda, &data).map(Some)
    }

    // --- Swap ---
    pub fn get_swap_pool(
        &self,
        token_a_mint: &str,
        token_b_mint: &str,
    ) -> TribeResult<Option<LiquidityPoolInfo>> {
        let a = Pubkey::from_str(token_a_mint).map_err(|e| {
            pot_o_core::TribeError::ChainBridgeError(format!("invalid token_a: {e}"))
        })?;
        let b = Pubkey::from_str(token_b_mint).map_err(|e| {
            pot_o_core::TribeError::ChainBridgeError(format!("invalid token_b: {e}"))
        })?;
        let (pda, _) =
            Pubkey::find_program_address(&[b"pool", a.as_ref(), b.as_ref()], &self.swap_program_id);
        let data = match self.get_account(&pda) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        parse_liquidity_pool(&pda, &data).map(Some)
    }

    pub fn get_swap_quote(
        &self,
        token_a_mint: &str,
        token_b_mint: &str,
        amount_in: u64,
        is_a_to_b: bool,
    ) -> TribeResult<Option<SwapQuoteInfo>> {
        let pool = match self.get_swap_pool(token_a_mint, token_b_mint)? {
            Some(p) => p,
            None => return Ok(None),
        };
        let (reserve_in, reserve_out) = if is_a_to_b {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };
        let amount_out = calc_swap_output(amount_in, reserve_in, reserve_out, pool.swap_fee_bps);
        let fee = calc_fee(amount_in, pool.swap_fee_bps);
        let price_impact_bps = calc_price_impact_bps(amount_in, reserve_in);
        Ok(Some(SwapQuoteInfo {
            pool: pool.pubkey,
            amount_in,
            amount_out,
            fee,
            price_impact_bps,
        }))
    }

    // --- Vault ---
    pub fn get_treasury(&self, token_mint: &str) -> TribeResult<Option<TreasuryInfo>> {
        let mint = Pubkey::from_str(token_mint)
            .map_err(|e| pot_o_core::TribeError::ChainBridgeError(format!("invalid mint: {e}")))?;
        let (pda, _) =
            Pubkey::find_program_address(&[b"treasury", mint.as_ref()], &self.vault_program_id);
        let data = match self.get_account(&pda) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        parse_treasury(&pda, &data).map(Some)
    }

    pub fn get_user_vault(
        &self,
        treasury_pubkey: &str,
        user_pubkey: &str,
    ) -> TribeResult<Option<UserVaultInfo>> {
        let treasury = Pubkey::from_str(treasury_pubkey).map_err(|e| {
            pot_o_core::TribeError::ChainBridgeError(format!("invalid treasury: {e}"))
        })?;
        let user = Pubkey::from_str(user_pubkey)
            .map_err(|e| pot_o_core::TribeError::ChainBridgeError(format!("invalid user: {e}")))?;
        let (pda, _) = Pubkey::find_program_address(
            &[b"user_vault", treasury.as_ref(), user.as_ref()],
            &self.vault_program_id,
        );
        let data = match self.get_account(&pda) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        parse_user_vault(&pda, &data).map(Some)
    }

    pub fn get_escrow(
        &self,
        depositor: &str,
        beneficiary: &str,
    ) -> TribeResult<Option<EscrowInfo>> {
        let dep = Pubkey::from_str(depositor).map_err(|e| {
            pot_o_core::TribeError::ChainBridgeError(format!("invalid depositor: {e}"))
        })?;
        let ben = Pubkey::from_str(beneficiary).map_err(|e| {
            pot_o_core::TribeError::ChainBridgeError(format!("invalid beneficiary: {e}"))
        })?;
        let (pda, _) = Pubkey::find_program_address(
            &[b"escrow", dep.as_ref(), ben.as_ref()],
            &self.vault_program_id,
        );
        let data = match self.get_account(&pda) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        parse_escrow(&pda, &data).map(Some)
    }
}
