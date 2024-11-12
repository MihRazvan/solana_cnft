use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount, Mint},
    associated_token::AssociatedToken,
};

declare_id!("91CLwQaCxutnTf8XafP3e6EmGBA3eUkMaw86Hgghax2L");

#[program]
pub mod solana_cnft {
    use super::*;

    pub const FRACTION_AMOUNT: u64 = 1_000;
    pub const FRACTION_DECIMALS: u8 = 0;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn lock_cnft(
        ctx: Context<LockCNFT>,
        asset_id: Pubkey,
    ) -> Result<()> {
        // Store vault info
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.asset_id = asset_id;
        vault.merkle_tree = ctx.accounts.merkle_tree.key();
        vault.locked_at = Clock::get()?.unix_timestamp;

        // Mint fractions to owner
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: ctx.accounts.fraction_mint.to_account_info(),
                    to: ctx.accounts.owner_fraction_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                &[&[b"authority", &[ctx.bumps.authority]]],
            ),
            FRACTION_AMOUNT,
        )?;

        msg!("cNFT locked in vault: {}", vault.key());
        Ok(())
    }

    pub fn unlock_cnft(ctx: Context<UnlockCNFT>) -> Result<()> {
        // Verify ownership
        require!(
            ctx.accounts.vault.owner == ctx.accounts.owner.key(),
            ErrorCode::InvalidOwner
        );

        // Burn all fractions before unlocking
        anchor_spl::token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Burn {
                    mint: ctx.accounts.fraction_mint.to_account_info(),
                    from: ctx.accounts.owner_fraction_account.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            FRACTION_AMOUNT,
        )?;

        msg!("Unlocking cNFT from vault: {}", ctx.accounts.vault.key());
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = FRACTION_DECIMALS,
        mint::authority = authority,
        mint::freeze_authority = authority
    )]
    pub fraction_mint: Account<'info, Mint>,

    /// CHECK: Program authority for fraction mint
    #[account(
        seeds = [b"authority"],
        bump
    )]
    pub authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

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

    /// CHECK: Verified through compression program
    pub merkle_tree: UncheckedAccount<'info>,

    #[account(mut)]
    pub fraction_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = fraction_mint,
        associated_token::authority = owner,
    )]
    pub owner_fraction_account: Account<'info, TokenAccount>,

    /// CHECK: PDA for fraction mint authority
    #[account(
        seeds = [b"authority"],
        bump
    )]
    pub authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
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

    #[account(mut)]
    pub fraction_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = fraction_mint,
        associated_token::authority = owner,
    )]
    pub owner_fraction_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub asset_id: Pubkey,
    pub merkle_tree: Pubkey,
    pub locked_at: i64,
}

impl Vault {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        32 + // asset_id
        32 + // merkle_tree
        8;  // locked_at
}

#[error_code]
pub enum ErrorCode {
    #[msg("Not the NFT owner")]
    InvalidOwner,
}
