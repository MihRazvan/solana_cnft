use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token},
    associated_token::AssociatedToken,
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = crate::solana_cnft::FRACTION_DECIMALS,
        mint::authority = authority,
        mint::freeze_authority = authority
    )]
    pub fraction_mint: Account<'info, Mint>,

    /// CHECK: Program authority for fraction mint
    #[account(
        seeds = [crate::solana_cnft::AUTHORITY_PREFIX],
        bump
    )]
    pub authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    Ok(())
}