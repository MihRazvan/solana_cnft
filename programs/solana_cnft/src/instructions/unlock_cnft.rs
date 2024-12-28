use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint, burn};
use mpl_bubblegum::ID as BUBBLEGUM_ID;
use spl_account_compression::{
    ID as COMPRESSION_ID,
    Noop,
};

use crate::{
    error::ErrorCode,
    state::Vault,
    utils::calculate_fraction_amount,
};

#[derive(Accounts)]
pub struct UnlockCNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        close = owner,
        seeds = [
            crate::solana_cnft::VAULT_PREFIX,
            merkle_tree.key().as_ref(),
            &vault.nonce.to_le_bytes()
        ],
        bump,
        has_one = owner,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        seeds = [merkle_tree.key().as_ref()],
        seeds::program = BUBBLEGUM_ID,
        bump,
    )]
    /// CHECK: Validated through seeds
    pub tree_authority: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated through CPI
    pub merkle_tree: UncheckedAccount<'info>,

    #[account(mut)]
    pub fraction_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = fraction_mint,
        associated_token::authority = owner,
    )]
    pub owner_fraction_account: Account<'info, TokenAccount>,

    /// CHECK: Bubblegum program
    pub bubblegum_program: Program<'info, Noop>,
    pub log_wrapper: Program<'info, Noop>,
    /// CHECK: Compression program
    pub compression_program: Program<'info, Noop>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<UnlockCNFT>) -> Result<()> {
    let vault = &ctx.accounts.vault;
    
    // Verify fraction token balance
    let fraction_amount = calculate_fraction_amount(&vault.data_hash, &vault.creator_hash);
    require!(
        ctx.accounts.owner_fraction_account.amount >= fraction_amount,
        ErrorCode::InsufficientFractionBalance
    );

  // Transfer NFT back using CPI
crate::utils::transfer_compressed_nft(
    &ctx.accounts.bubblegum_program.to_account_info(),
    &ctx.accounts.tree_authority.to_account_info(),
    &vault.to_account_info(),
    ctx.accounts.owner.key(),
    &ctx.accounts.merkle_tree.to_account_info(),
    &ctx.accounts.log_wrapper.to_account_info(),
    &ctx.accounts.compression_program.to_account_info(),
    vault.root,
    vault.data_hash,
    vault.creator_hash,
    vault.nonce,
    vault.index,
    ctx.remaining_accounts,
)?;

    // Burn fractions
    burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Burn {
                mint: ctx.accounts.fraction_mint.to_account_info(),
                from: ctx.accounts.owner_fraction_account.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        fraction_amount,
    )?;

    msg!("cNFT unlocked from vault: {}", vault.key());
    Ok(())
}