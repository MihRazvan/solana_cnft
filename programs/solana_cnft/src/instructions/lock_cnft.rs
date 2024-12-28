use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount, Mint, mint_to},
    associated_token::AssociatedToken,
};
use mpl_bubblegum::{
    programs::{MPL_BUBBLEGUM_ID, SPL_NOOP_ID},
    accounts::TreeConfig,
    types::{MetadataArgs, LeafSchema},
    hash::hash_metadata,
};
use spl_account_compression::{
    programs::SPL_ACCOUNT_COMPRESSION_ID,
    Noop,
};

#[derive(Accounts, Bumps)]
#[instruction(asset_id: Pubkey)]
pub struct LockCNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = Vault::LEN,
        seeds = [
            crate::solana_cnft::VAULT_PREFIX,
            merkle_tree.key().as_ref(),
            &nonce.to_le_bytes()
        ],
        bump
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
        init_if_needed,
        payer = owner,
        associated_token::mint = fraction_mint,
        associated_token::authority = owner,
    )]
    pub owner_fraction_account: Account<'info, TokenAccount>,

    /// CHECK: PDA for fraction mint authority
    #[account(
        seeds = [crate::solana_cnft::AUTHORITY_PREFIX],
        bump
    )]
    pub authority: UncheckedAccount<'info>,

    pub bubblegum_program: Program<'info, mpl_bubblegum::program::Bubblegum>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<LockCNFT>,
    metadata: MetadataArgs,
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.owner = ctx.accounts.owner.key();
    vault.merkle_tree = ctx.accounts.merkle_tree.key();
    vault.root = root;
    vault.data_hash = data_hash;
    vault.nonce = nonce;
    vault.index = index;

    let asset_id = get_asset_id(&ctx.accounts.merkle_tree.key(), nonce);
    
    // Transfer NFT to vault using Bubblegum CPI
    let cpi_accounts = mpl_bubblegum::cpi::accounts::Transfer {
        tree_authority: ctx.accounts.tree_authority.to_account_info(),
        leaf_owner: ctx.accounts.owner.to_account_info(), 
        leaf_delegate: ctx.accounts.owner.to_account_info(),
        new_leaf_owner: vault.key(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        compression_program: ctx.accounts.compression_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
        ctx.accounts.bubblegum_program.to_account_info(),
        cpi_accounts,
    );

    mpl_bubblegum::cpi::transfer(cpi_ctx, root, data_hash, creator_hash, nonce, index)?;

    // Mint fractions
    let mint_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        anchor_spl::token::MintTo {
            mint: ctx.accounts.fraction_mint.to_account_info(),
            to: ctx.accounts.owner_fraction_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        },
        &[&[AUTHORITY_PREFIX, &[ctx.bumps.authority]]]
    );
    
    let fraction_amount = calculate_fraction_amount(&data_hash, &creator_hash);
    anchor_spl::token::mint_to(mint_ctx, fraction_amount)?;

    Ok(())
}