use anchor_lang::prelude::*;

#[account]
pub struct Vault {
    pub admin: Pubkey,           // Admin of the vault
    pub shares_mint: Pubkey,     // SPL mint for vault shares
    pub base_asset_mint: Pubkey, // SPL mint accepted for deposits
    pub total_base_assets: u64,  // Total amount of base asset managed by the vault
    pub deposit_paused: bool,    // Flag to pause deposits
    pub allocate_paused: bool,   // Flag to pause allocations
    pub bump: u8,
}

impl Vault {
    pub const MAX_SIZE: usize = 32 + // Pubkey: admin
        32 + //Pubkey: shares_mint
        32 + // Pubkey: base_mint
        8 +  // u64: total_base_assets
        1 +  // bool: deposit_paused
        1 +  // bool: allocate_paused
        1; // u8: bump

    pub fn initialize(
        &mut self,
        admin: Pubkey,
        shares_mint: Pubkey,
        base_asset_mint: Pubkey,
        bump: u8,
    ) -> Result<()> {
        self.admin = admin;
        self.shares_mint = shares_mint;
        self.base_asset_mint = base_asset_mint;
        self.total_base_assets = 0;
        self.deposit_paused = false;
        self.allocate_paused = false;
        self.bump = bump;

        Ok(())
    }
}
