use anchor_lang::prelude::*;
use bytemuck::Zeroable;

#[account(zero_copy)]
#[repr(C)]
pub struct Vault {
    pub admin: Pubkey,                // Admin of the vault
    pub vault_authority: Pubkey,      // Vault authority address
    pub shares_mint: Pubkey,          // SPL mint for vault shares
    pub base_asset_mint: Pubkey,      // SPL mint accepted for deposits
    pub vault_base_asset_ata: Pubkey, // SPL vault base token associated account
    pub token_program: Pubkey,        // Token program address
    pub total_base_assets: u64,       // Total amount of base asset managed by the vault
    pub mint_shares_decimals: u8,     // Mint shares decimals
    pub deposit_paused: u8,       // Flag to pause deposits
    pub allocate_paused: u8,      // Flag to pause allocations
    pub vault_authority_bump: u8,     // vault authority bump
    pub mint_shares_bump: u8,         // vault authority bump
    pub _padding: [u8; 3],            //padding for alignment
}

impl Vault {
    pub const MAX_SIZE: usize = 32 + // Pubkey: admin
        32 + //Pubkey: shares_mint
        32 + //Pubkey: vault_authority
        32 + // Pubkey: base_mint
        32 + // Pubkey: vault base asset ata
        32 + // Pubkey: token program address
        8 +  // u64: total_base_assets
        1 +  // u64: mint shares decimals
        1 +  // u8: deposit_paused
        1 + // u8: allocate_paused
        1 + // u8: vault authority bump
        1 + // u8: mint shares bump
        3; // padding

    pub fn initialize(
        &mut self,
        admin: Pubkey,
        vault_authority: Pubkey,
        shares_mint: Pubkey,
        base_asset_mint: Pubkey,
        token_program: Pubkey,
        vault_base_asset_ata: Pubkey,
        mint_shares_decimals: u8,
        vault_authority_bump: u8,
        mint_shares_bump: u8,
    ) -> Result<()> {
        self.admin = admin;
        self.vault_authority = vault_authority;
        self.shares_mint = shares_mint;
        self.base_asset_mint = base_asset_mint;
        self.token_program = token_program;
        self.vault_base_asset_ata = vault_base_asset_ata;
        self.mint_shares_decimals = mint_shares_decimals;
        self.vault_authority_bump = vault_authority_bump;
        self.mint_shares_bump = mint_shares_bump;

        //default fields
        self.total_base_assets = 0;
        self.deposit_paused = 0;
        self.allocate_paused = 0;
        self._padding = [0; 3];

        Ok(())
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            admin: Pubkey::zeroed(),
            vault_authority: Pubkey::zeroed(),
            shares_mint: Pubkey::zeroed(),
            base_asset_mint: Pubkey::zeroed(),
            token_program: Pubkey::zeroed(),
            vault_base_asset_ata: Pubkey::zeroed(),
            mint_shares_decimals: 0,
            vault_authority_bump: 0,
            mint_shares_bump: 0,
            total_base_assets: 0,
            deposit_paused: 0,
            allocate_paused: 0,
            _padding: [0; 3],
        }
    }
}
