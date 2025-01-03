use anchor_lang::prelude::*;
use mpl_bubblegum::{
    ID as BUBBLEGUM_ID,
    types::MetadataArgs,
};
use spl_account_compression::ID as COMPRESSION_ID;

pub mod instructions;
pub mod state;
pub mod error;
pub mod utils;

use instructions::*;
use state::*;
use error::*;

declare_id!("91CLwQaCxutnTf8XafP3e6EmGBA3eUkMaw86Hgghax2L");

#[program]
pub mod solana_cnft {
    use super::*;
    use mpl_bubblegum::types::MetadataArgs; // Add this import inside the module

    pub const FRACTION_AMOUNT: u64 = 1_000;
    pub const FRACTION_DECIMALS: u8 = 0;
    pub const VAULT_PREFIX: &[u8] = b"vault";
    pub const AUTHORITY_PREFIX: &[u8] = b"authority";

    pub fn initialize<'info>(ctx: Context<'_, '_, '_, 'info, Initialize<'info>>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn lock_cnft<'info>(
        ctx: Context<'_, '_, '_, 'info, LockCNFT<'info>>,
        metadata: MetadataArgs,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        instructions::lock_cnft::handler(ctx, metadata, root, data_hash, creator_hash, nonce, index)
    }

    pub fn unlock_cnft<'info>(ctx: Context<'_, '_, '_, 'info, UnlockCNFT<'info>>) -> Result<()> {
        instructions::unlock_cnft::handler(ctx)
    }

    pub const AUTHORITY_BUMP: &[u8] = b"authority_bump";
}