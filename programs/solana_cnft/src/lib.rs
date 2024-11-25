use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, instruction::AccountMeta},
};
use anchor_spl::{
    token::{Token, TokenAccount, Mint},
    associated_token::AssociatedToken,
};

use mpl_bubblegum::{
    program::Bubblegum,
    state::{
        leaf_schema::LeafSchema,
        TreeConfig,
    },
    hash::{hash_metadata, hash_creators},
};

use spl_account_compression::{
    program::SplAccountCompression,
    wrap_application_data_v1,
    Noop,
    Node,
};

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

    pub const FRACTION_AMOUNT: u64 = 1_000;
    pub const FRACTION_DECIMALS: u8 = 0;
    pub const VAULT_PREFIX: &[u8] = b"vault";
    pub const AUTHORITY_PREFIX: &[u8] = b"authority";

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn lock_cnft(
        ctx: Context<LockCNFT>, 
        metadata: mpl_bubblegum::types::MetadataArgs,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        instructions::lock_cnft::handler(ctx, metadata, root, data_hash, creator_hash, nonce, index)
    }

    pub fn unlock_cnft(ctx: Context<UnlockCNFT>) -> Result<()> {
        instructions::unlock_cnft::handler(ctx)
    }
}