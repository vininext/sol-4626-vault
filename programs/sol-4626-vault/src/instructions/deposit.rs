use crate::constant::{VAULT_AUTHORITY_SEED};
use crate::state::Vault;
use crate::util::{convert_to_shares, Errors};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

/// Deposit accounts:
/// - signer: depositor
/// - shares_mint: vault's shares mint
/// - shares_ata: depositor's ATA for shares
/// - base_asset_mint: base token asset mint
/// - base_asset_ata: depositor's ATA holding base assets
/// - vault_base_asset_ata: vault's ATA for base assets
/// - vault: vault PDA
/// - token_program
/// - associated_token_program
/// - system_program
#[derive(Accounts)]
#[instruction(amount: u64, ticker: [u8; 16])]
pub struct Deposit<'info> {
    #[account(mut)]
    signer: Signer<'info>,
    #[account(mut)]
    shares_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut,
        has_one = shares_mint,
        has_one = base_asset_mint,
        has_one = vault_authority,
        has_one = vault_base_asset_ata,
        has_one = token_program
    )]
    vault: AccountLoader<'info, Vault>,
    /// CHECK: vault authority checked (has_one)
    #[account(mut)]
    vault_authority: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = shares_mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    shares_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account()]
    base_asset_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = base_asset_mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    base_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    vault_base_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    token_program: Interface<'info, TokenInterface>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

/// Process a deposit: validate amount, transfer base asset to vault, mint shares.
/// - amount: amount of base asset to deposit
pub fn handle(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let mut vlt = ctx.accounts.vault.load_mut()?;

    require!(
        ctx.accounts.base_asset_ata.amount >= amount,
        Errors::InsufficientBaseAssetBalance
    );
    require!(amount > 0, Errors::ZeroDeposit);
    require!(vlt.deposit_paused == 0, Errors::DepositPaused);

    msg!(
        "depositing {} base assets into vault {}",
        amount,
        ctx.accounts.vault.key()
    );

    let total_shares = ctx.accounts.shares_mint.supply;
    let total_assets = vlt.total_base_assets;

    //to be minted
    let to_mint = convert_to_shares(amount, total_assets, total_shares)?;

    // Transfer base assets from user to vault
    let transfer_accounts = TransferChecked {
        mint: ctx.accounts.base_asset_mint.to_account_info(),
        from: ctx.accounts.base_asset_ata.to_account_info(),
        to: ctx.accounts.vault_base_asset_ata.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        transfer_accounts,
    );
    transfer_checked(transfer_ctx, amount, ctx.accounts.base_asset_mint.decimals)?;

    // Mint shares to user
    let vlt_address = ctx.accounts.vault.key();
    let mint_accounts = MintTo {
        mint: ctx.accounts.shares_mint.to_account_info(),
        to: ctx.accounts.shares_ata.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    let vlt_auth_seeds: &[&[&[u8]]] = &[&[
        VAULT_AUTHORITY_SEED.as_bytes(),
        vlt_address.as_ref(),
        &[vlt.vault_authority_bump],
    ]];
    let mint_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        mint_accounts,
        vlt_auth_seeds,
    );
    mint_to(mint_ctx, to_mint)?;

    // Update vault state
    vlt.total_base_assets = vlt
        .total_base_assets
        .checked_add(amount)
        .ok_or(Errors::MathOverflow)?;

    emit!(DepositEvent {
        depositor: ctx.accounts.signer.key(),
        base_asset_amount: amount,
        shares_minted: to_mint,
    });

    Ok(())
}

#[event]
pub struct DepositEvent {
    pub depositor: Pubkey,
    pub base_asset_amount: u64,
    pub shares_minted: u64,
}
