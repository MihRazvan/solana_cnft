use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount, Mint, burn},
    associated_token::AssociatedToken,
};
use mpl_bubblegum::state::TreeConfig;
use spl_account_compression::{program::SplAccountCompression, wrap_application_data_v1, Noop};

use crate::{error::ErrorCode, state::Vault};

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
    /// CHECK: Validated by seeds
    pub tree_authority: Account<'info, TreeConfig>,

    /// CHECK: Validated through cpi
    #[account(mut)]
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
    let vault_key = vault.key();

    // Create leaf schema for previous and new state
    let previous_leaf = mpl_bubblegum::state::leaf_schema::LeafSchema::V1 {
        id: crate::utils::get_asset_id(&ctx.accounts.merkle_tree.key(), vault.nonce),
        owner: vault_key,
        delegate: vault_key,
        nonce: vault.nonce,
        data_hash: vault.data_hash,
        creator_hash: vault.creator_hash,
    };

    let new_leaf = mpl_bubblegum::state::leaf_schema::LeafSchema::V1 {
        id: crate::utils::get_asset_id(&ctx.accounts.merkle_tree.key(), vault.nonce),
        owner: ctx.accounts.owner.key(),
        delegate: ctx.accounts.owner.key(),
        nonce: vault.nonce,
        data_hash: vault.data_hash,
        creator_hash: vault.creator_hash,
    };

    // Log state change
    wrap_application_data_v1(
        new_leaf.try_to_vec()?,
        &ctx.accounts.log_wrapper.to_account_info(),
    )?;

    // Transfer NFT back to owner
    crate::utils::transfer_compressed_nft(
        ctx.accounts.bubblegum_program.to_account_info(),
        ctx.accounts.tree_authority.to_account_info(),
        vault_key,
        ctx.accounts.owner.key(),
        ctx.accounts.merkle_tree.to_account_info(),
        ctx.accounts.log_wrapper.to_account_info(),
        ctx.accounts.compression_program.to_account_info(),
        vault.root,
        previous_leaf.to_node(),
        new_leaf.to_node(),
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
        crate::solana_cnft::FRACTION_AMOUNT,
    )?;

    msg!("Unlocking cNFT from vault: {}", vault_key);
    Ok(())
}