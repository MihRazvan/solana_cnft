use mpl_bubblegum::{
    accounts::TreeConfig,
    programs::{MPL_BUBBLEGUM_ID, SPL_NOOP_ID},
    types::LeafSchema,
};
use spl_account_compression::{program::ID as COMPRESSION_ID, Noop, Node};

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
    let vault_key = vault.key();
    
    // Verify owner has all fractions
    let fraction_amount = crate::utils::calculate_fraction_amount(&vault.data_hash, vault.creator_hash);
    require!(
        ctx.accounts.owner_fraction_account.amount >= fraction_amount,
        ErrorCode::InsufficientFractionBalance
    );

    // Create leaf schema for transfer
    let asset_id = crate::utils::get_asset_id(&ctx.accounts.merkle_tree.key(), vault.nonce);
    let previous_leaf = LeafSchema::new_v0(
        asset_id,
        vault_key,
        vault_key,
        vault.nonce,
        vault.data_hash,
        vault.creator_hash,
    );

    let new_leaf = LeafSchema::new_v0(
        asset_id,
        ctx.accounts.owner.key(),
        ctx.accounts.owner.key(),
        vault.nonce,
        vault.data_hash,
        vault.creator_hash,
    );

    // Log state change
    wrap_application_data_v1(
        new_leaf.to_event().try_to_vec()?,
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
        fraction_amount,
    )?;

    msg!("Unlocking cNFT from vault: {}", vault_key);
    Ok(())
}