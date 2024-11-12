use anchor_lang::prelude::*;

declare_id!("91CLwQaCxutnTf8XafP3e6EmGBA3eUkMaw86Hgghax2L");

#[program]
pub mod solana_cnft {
    use super::*;

    pub const FRACTION_AMOUNT: u64 = 1_000;  // Fixed number of fractions
    pub const FRACTION_DECIMALS: u8 = 0;     // No decimals for simplicity

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn lock_cnft(
        ctx: Context<LockCNFT>,
        asset_id: Pubkey,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.asset_id = asset_id;
        vault.merkle_tree = ctx.accounts.merkle_tree.key();
        vault.fraction_mint = ctx.accounts.fraction_mint.key();
        vault.fraction_amount = FRACTION_AMOUNT;
        vault.locked_at = Clock::get()?.unix_timestamp;

        msg!("cNFT locked in vault: {}", vault.key());
        Ok(())
    }

    pub fn unlock_cnft(ctx: Context<UnlockCNFT>) -> Result<()> {
        require!(
            ctx.accounts.vault.owner == ctx.accounts.owner.key(),
            ErrorCode::InvalidOwner
        );

        msg!("Unlocking cNFT from vault: {}", ctx.accounts.vault.key());
        Ok(())
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Not the NFT owner")]
    InvalidOwner,
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

    /// CHECK: This account should be the merkle tree that contains our cNFT
    pub merkle_tree: UncheckedAccount<'info>,

    /// Will be initialized in our next step for fraction tokens
    /// CHECK: Verified through mint creation
    #[account(mut)]
    pub fraction_mint: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnlockCNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        close = owner,
        seeds = [b"vault", vault.asset_id.as_ref()],
        bump,
        has_one = owner,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub asset_id: Pubkey,
    pub merkle_tree: Pubkey,
    pub fraction_mint: Pubkey,    // Added to track fraction token mint
    pub fraction_amount: u64,     // Track total supply of fractions
    pub locked_at: i64,
}

impl Vault {
    pub const LEN: usize = 8 +  // discriminator
        32 + // owner
        32 + // asset_id
        32 + // merkle_tree
        32 + // fraction_mint
        8 +  // fraction_amount
        8;   // locked_at
}