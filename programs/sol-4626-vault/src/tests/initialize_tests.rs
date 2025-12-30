#[cfg(test)]
mod test_initialize {
    use crate::constant::{SHARES_MINT_SEED, VAULT_AUTHORITY_SEED};
    use crate::state::Vault;
    use crate::ID;
    use anchor_lang::prelude::Pubkey;
    use anchor_lang::{system_program, Id};
    use anchor_spl::associated_token::{get_associated_token_address, AssociatedToken};
    use litesvm::LiteSVM;
    use litesvm_token::{CreateMint, TOKEN_ID};
    use sha2::{Digest, Sha256};
    use solana_sdk::account::Account;
    use solana_sdk::message::{AccountMeta, Address, Instruction};
    use solana_sdk::native_token::LAMPORTS_PER_SOL;
    use solana_sdk::signature::{Keypair, Signer};
    use solana_sdk::transaction::Transaction;

    #[test]
    pub fn test_initialize_success() {
        let mut svm = LiteSVM::new();

        //adding our program to svm
        let program_id = ID;
        svm.add_program_from_file(
            program_id.to_bytes(),
            "../../target/deploy/sol_4626_vault.so",
        )
        .unwrap();

        //airdrop payer
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        //creating mint
        let base_asset_mint = CreateMint::new(&mut svm, &payer)
            .decimals(9)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        //preparing accounts
        let admin_kp = Keypair::new();
        let vlt_kp = Keypair::new();
        let admin = admin_kp.pubkey();
        let vlt = vlt_kp.pubkey();

        //vlt authority
        let vlt_authority_seeds = &[VAULT_AUTHORITY_SEED.as_bytes(), vlt.as_ref()];
        let (vlt_authority, vlt_bump) =
            Pubkey::find_program_address(vlt_authority_seeds, &program_id);

        //shares mint
        let shares_mint_seeds = &[SHARES_MINT_SEED.as_bytes(), vlt_authority.as_ref()];
        let (shares_mint, shares_mint_bump) =
            Pubkey::find_program_address(shares_mint_seeds, &program_id);

        //vault base asset ata
        let vault_base_asset_ata: Pubkey = get_associated_token_address(
            &vlt_authority,
            &Pubkey::new_from_array(base_asset_mint.to_bytes()),
        );

        //airdrop admin
        svm.airdrop(&admin, LAMPORTS_PER_SOL * 2).unwrap();

        //init vault data (zero)
        let fn_disc = Sha256::digest(b"global:initialize");
        let data = vec![0; 8 + Vault::MAX_SIZE];
        let rent = svm.minimum_balance_for_rent_exemption(8 + Vault::MAX_SIZE);

        let vlt_acc = Account {
            lamports: rent,
            data,
            owner: Address::new_from_array(program_id.to_bytes()),
            executable: false,
            rent_epoch: 0,
        };

        //create zeroed vlt account
        svm.set_account(vlt, vlt_acc).unwrap();

        //define accounts for ix
        let accs = vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(vlt, false),
            AccountMeta::new_readonly(Address::from(vlt_authority.to_bytes()), false),
            AccountMeta::new_readonly(base_asset_mint, false),
            AccountMeta::new(
                Address::new_from_array(vault_base_asset_ata.to_bytes()),
                false,
            ),
            AccountMeta::new(Address::from(shares_mint.to_bytes()), false),
            AccountMeta::new_readonly(TOKEN_ID, false),
            AccountMeta::new_readonly(
                Address::new_from_array(AssociatedToken::id().to_bytes()),
                false,
            ),
            AccountMeta::new_readonly(
                Address::new_from_array(system_program::ID.to_bytes()),
                false,
            ),
        ];

        //build ix
        let ix = Instruction::new_with_bytes(
            Address::new_from_array(program_id.to_bytes()),
            &fn_disc[..8],
            accs,
        );

        //build tx
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin),
            &[admin_kp],
            svm.latest_blockhash(),
        );

        //send tx
        match svm.send_transaction(tx) {
            Ok(tx) => println!("tx has been successfully sent sig: {}", tx.signature),
            Err(err) => panic!(
                "error sending tx. err: {} meta {:?}",
                err.err, err.meta.logs
            ),
        }

        //load vlt account
        let vlt_acc = svm.get_account(&vlt).expect("vault account not found");

        assert_eq!(vlt_acc.owner.to_bytes()[..], program_id.to_bytes()[..]);
    }
}
