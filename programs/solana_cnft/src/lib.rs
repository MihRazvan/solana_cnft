use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Mint, Token, TokenAccount},
    associated_token::AssociatedToken,
};

declare_id!("91CLwQaCxutnTf8XafP3e6EmGBA3eUkMaw86Hgghax2L");

#[program]
pub mod solana_cnft {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn lock_cnft(
        ctx: Context<LockCNFT>,
        asset_id: Pubkey,
    ) -> Result<()> {
        // Store information about the locked cNFT
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.asset_id = asset_id;
        vault.locked_at = Clock::get()?.unix_timestamp;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
#[instruction(asset_id: Pubkey)]
pub struct LockCNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = Vault::LEN,
        seeds = [b"vault", asset_id.as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub asset_id: Pubkey,
    pub locked_at: i64,
}

impl Vault {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        32 + // asset_id
        8;  // locked_at
}