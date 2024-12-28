use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint, burn};
use mpl_bubblegum::{
    programs::{MPL_BUBBLEGUM_ID, SPL_NOOP_ID},
    accounts::TreeConfig,
    types::LeafSchema,
    instructions::TransferCpiBuilder,
};
use spl_account_compression::{
    programs::SPL_ACCOUNT_COMPRESSION_ID,
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
        seeds::program = bubblegum_program.key(),
        bump,
    )]
    pub tree_authority: Account<'info, TreeConfig>,

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

    pub bubblegum_program: Program<'info, mpl_bubblegum::program::Bubblegum>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
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

    // Transfer NFT back to owner using CPI
    let transfer = TransferCpiBuilder::new(ctx.accounts.bubblegum_program.to_account_info())
        .tree_authority(ctx.accounts.tree_authority.to_account_info())
        .leaf_owner(vault.to_account_info())
        .leaf_delegate(vault.to_account_info())
        .new_leaf_owner(ctx.accounts.owner.key())
        .merkle_tree(ctx.accounts.merkle_tree.to_account_info())
        .log_wrapper(ctx.accounts.log_wrapper.to_account_info())
        .compression_program(ctx.accounts.compression_program.to_account_info())
        .system_program(ctx.accounts.system_program.to_account_info())
        .root(vault.root)
        .data_hash(vault.data_hash)
        .creator_hash(vault.creator_hash)
        .nonce(vault.nonce)
        .index(vault.index);

    // Add proof accounts
    for account in ctx.remaining_accounts.iter() {
        transfer.add_remaining_account(account, false, false);
    }

    transfer.invoke()?;

    // Burn fraction tokens
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