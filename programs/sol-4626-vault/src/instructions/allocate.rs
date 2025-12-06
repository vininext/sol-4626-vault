use crate::state::Vault;
use crate::util::Errors;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};
/// Allocate accounts:
/// - signer: vault admin
/// - base_asset_mint: vault's base asset mint
/// - vault_base_asset_ata: vault's ATA for base assets
/// - vault: vault PDA
/// - target_ata: external target ATA to allocate assets to
/// - token_program
/// - system_program
#[derive(Accounts)]
pub struct Allocate<'info> {
    #[account(mut, address = vault.admin)]
    signer: Signer<'info>,
    #[account(
        mint::token_program = token_program,
        address = vault.base_asset_mint
    )]
    base_asset_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = base_asset_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    vault_base_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        seeds = [b"vault"],
        bump
    )]
    vault: Box<Account<'info, Vault>>,
    #[account(
        mut,
        token::mint = base_asset_mint,
        token::token_program = token_program,
    )]
    target_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

/// Moves base assets from the vault's ATA to an external target ATA.
/// The vault's accounting keeps total assets unchanged because funds
/// are only being relocated (e.g., allocated to an external yield strategy),
pub fn handle(ctx: Context<Allocate>, amount: u64) -> Result<()> {
    require!(amount > 0, Errors::InvalidAmount);
    require!(!ctx.accounts.vault.allocate_paused, Errors::AllocatePaused);
    require!(
        amount <= ctx.accounts.vault_base_asset_ata.amount,
        Errors::InsufficientBaseAssetBalance
    );

    msg!(
        "allocating {} base assets from vault {} to target ATA {}",
        amount,
        ctx.accounts.vault.key(),
        ctx.accounts.target_ata.key()
    );

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.vault_base_asset_ata.to_account_info(),
        mint: ctx.accounts.base_asset_mint.to_account_info(),
        to: ctx.accounts.target_ata.to_account_info(),
        authority: ctx.accounts.vault.to_account_info(),
    };

    let signer_seeds: &[&[&[u8]]] = &[&[b"vault", &[ctx.bumps.vault]]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        transfer_accounts,
        signer_seeds,
    );

    transfer_checked(transfer_ctx, amount, ctx.accounts.base_asset_mint.decimals)?;

    emit!(AllocateEvent {
        vault: ctx.accounts.vault.key(),
        target_ata: ctx.accounts.target_ata.key(),
        amount,
    });

    Ok(())
}

#[event]
pub struct AllocateEvent {
    pub vault: Pubkey,
    pub target_ata: Pubkey,
    pub amount: u64,
}
