import * as anchor from "@coral-xyz/anchor";
import {BN, Program} from "@coral-xyz/anchor";
import {Sol4626Vault} from "../target/types/sol_4626_vault";
import {createMint, getMint, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import {Keypair, LAMPORTS_PER_SOL, PublicKey} from "@solana/web3.js";
import {assert} from "chai";

const VAULT_SEED = "vault";
const SHARES_MINT_SEED = "shares_mint";

describe("sol4626 end-to-end tests", () => {
    anchor.setProvider(anchor.AnchorProvider.env());

    // Initialize program and provider
    const program = anchor.workspace.Sol4626Vault as Program<Sol4626Vault>;
    const conn = program.provider.connection;

    //create test users
    const tokenCreator = Keypair.generate();
    const signer = Keypair.generate();
    const depositor = Keypair.generate();
    const yieldTarget = Keypair.generate();
    const poorUser = Keypair.generate();

    let baseAssetMint: PublicKey;

    before(async () => {
        //request airdrop for token creator and signer
        await conn.requestAirdrop(tokenCreator.publicKey, LAMPORTS_PER_SOL * 10);
        await conn.requestAirdrop(signer.publicKey, LAMPORTS_PER_SOL * 10);
        await conn.requestAirdrop(depositor.publicKey, LAMPORTS_PER_SOL * 10);
        await conn.requestAirdrop(yieldTarget.publicKey, LAMPORTS_PER_SOL * 10);
        await conn.requestAirdrop(poorUser.publicKey, LAMPORTS_PER_SOL * 0.1);

        // Wait for airdrops to complete
        await new Promise(resolve => setTimeout(resolve, 1000));

        // Create base mint once and store the public key
        baseAssetMint = await createMint(
            program.provider.connection,
            tokenCreator,
            tokenCreator.publicKey,
            tokenCreator.publicKey,
            6
        );

        // Wait for mint creation to finalize
        await new Promise(resolve => setTimeout(resolve, 500));

        // Create depositor ATA for the base mint
        const depositorBaseAssetMintAta = await getOrCreateAssociatedTokenAccount(
            program.provider.connection,
            depositor,
            baseAssetMint,
            depositor.publicKey
        );

        // Wait for creating ata to finalize
        await new Promise(resolve => setTimeout(resolve, 500));

        //Mint some tokens to signer
        await mintTo(
            conn,
            depositor,
            baseAssetMint,
            depositorBaseAssetMintAta.address,
            tokenCreator,
            10_000_000 // 10 Custom Tokens (6 decimals)
        )
    });

    it("should initialize the vault properly", async () => {
        // Accounts expected by Initialize:
        // 1. admin          → Signer paying for the vault + shares_mint creation.
        // 2. vault          → PDA holding vault config (admin, shares_mint, base_asset_mint).
        // 3. base_asset_mint→ Existing SPL mint for the underlying asset (e.g. USDC).
        // 4. vault_base_asset_ata → PDA's associated token account for holding base assets.
        // 5. shares_mint    → New SPL mint for vault shares (decimals = base_asset_mint.decimals,
        // 6. token_program  → SPL Token / Token-2022 program used for mint initialization.
        // 7. associated_token_program → Program for creating the vault's ATA.
        // 8. system_program → System program for account creation.
        try {
            const sig = await program.methods.initialize().accounts({
                admin: signer.publicKey,
                baseAssetMint: baseAssetMint,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
                .signers([signer])
                .rpc();

            console.log(
                `Your transaction signature: https://explorer.solana.com/transaction/${sig}?cluster=custom&customUrl=${conn.rpcEndpoint}`
            );
        } catch (err) {
            console.error("Transaction error: ", err);
            throw err;
        }

        await new Promise(resolve => {
            setTimeout(resolve, 200)
        })

        const [vltPDA] =
            PublicKey.findProgramAddressSync([Buffer.from(VAULT_SEED)], program.programId)
        const [sharesMintPDA] =
            PublicKey.findProgramAddressSync([Buffer.from(SHARES_MINT_SEED)], program.programId)

        const vlt = await program.account.vault.fetch(vltPDA);

        assert(vlt.admin.toString() == signer.publicKey.toString(), "invalid vault admin");
        assert(vlt.baseAssetMint.toString() == baseAssetMint.toString(), "invalid base asset mint");
        assert(vlt.sharesMint.toString() == sharesMintPDA.toString(), "invalid shares mint");
    });

    it("should make first deposit and mint shares", async () => {
        // Accounts expected by Deposit:
        // 1. signer → User depositing base assets; pays for ATA creation.
        // 2. shares_mint → Vault share mint; must equal vault.shares_mint.
        // 3. shares_ata → Signer's ATA to receive minted shares.
        // 4. base_asset_mint → Underlying asset mint; must equal vault.base_asset_mint.
        // 5. base_asset_ata → Signer's ATA holding the base asset to deposit.
        // 6. vault_base_asset_ata → Vault PDA’s ATA that receives deposited assets.
        // 7. vault → PDA with vault configuration and accounting.
        // 8. token_program → SPL Token or Token-2022 program.
        // 9. associated_token_program → Creates ATAs if missing.
        // 10. system_program → Required for system-level account creation.
        const [mintSharesPDA] = PublicKey.findProgramAddressSync([Buffer.from(SHARES_MINT_SEED)], program.programId)
        const [vltPDA] = PublicKey.findProgramAddressSync([Buffer.from(VAULT_SEED)], program.programId)

        try {
            const sig = await program.methods.deposit(new BN(5_000_000)).accounts({
                signer: depositor.publicKey,
                sharesMint: mintSharesPDA,
                baseAssetMint: baseAssetMint,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
                .signers([depositor])
                .rpc()

            console.log(
                `Your transaction signature: https://explorer.solana.com/transaction/${sig}?cluster=custom&customUrl=${conn.rpcEndpoint}`
            );

            const depositorMintAta = await getOrCreateAssociatedTokenAccount(
                program.provider.connection,
                depositor,
                mintSharesPDA,
                depositor.publicKey
            )

            const mintInfo = await getMint(
                program.provider.connection,
                mintSharesPDA
            )

            const vlt = await program.account.vault.fetch(vltPDA);

            assert(mintInfo.supply == BigInt(5_000_000), "invalid shares minted to signer");
            assert(depositorMintAta.amount == BigInt(5_000_000), "invalid shares in depositor's ATA");
            assert(vlt.totalBaseAssets == BigInt(5_000_000), "invalid total base assets in vault");
        } catch (err) {
            console.error("Transaction error: ", err);
            throw err;
        }
    });

    it("should mint proportional shares on second deposit", async () => {
        const [mintSharesPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from(SHARES_MINT_SEED)],
            program.programId
        );
        const [vltPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from(VAULT_SEED)],
            program.programId
        );

        // Accounts expected by Deposit:
        // 1. signer → User depositing base assets; pays for ATA creation.
        // 2. shares_mint → Vault share mint; must equal vault.shares_mint.
        // 3. shares_ata → Signer's ATA to receive minted shares.
        // 4. base_asset_mint → Underlying asset mint; must equal vault.base_asset_mint.
        // 5. base_asset_ata → Signer's ATA holding the base asset to deposit.
        // 6. vault_base_asset_ata → Vault PDA’s ATA that receives deposited assets.
        // 7. vault → PDA with vault configuration and accounting.
        // 8. token_program → SPL Token or Token-2022 program.
        // 9. associated_token_program → Creates ATAs if missing.
        // 10. system_program → Required for system-level account creation.
        try {
            const sig = await program.methods
                .deposit(new BN(5_000_000))
                .accounts({
                    signer: depositor.publicKey,
                    sharesMint: mintSharesPDA,
                    baseAssetMint: baseAssetMint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                })
                .signers([depositor])
                .rpc();

            console.log(
                `Your transaction signature: https://explorer.solana.com/transaction/${sig}?cluster=custom&customUrl=${conn.rpcEndpoint}`
            );
        } catch (err) {
            console.error("Transaction error: ", err);
            throw err;
        }

        await new Promise(resolve => {
            setTimeout(resolve, 200)
        })

        // Shares mint info
        const mintInfo = await getMint(conn, mintSharesPDA);

        // Depositor's shares ATA
        const depositorMintAta = await getOrCreateAssociatedTokenAccount(
            conn,
            depositor,
            mintSharesPDA,
            depositor.publicKey
        );

        const vlt = await program.account.vault.fetch(vltPDA);

        assert(mintInfo.supply == BigInt(10_000_000), "invalid total shares supply");
        assert(depositorMintAta.amount == BigInt(10_000_000), "invalid total shares in depositor ATA");
        assert(vlt.totalBaseAssets.toNumber() == 10_000_000, "invalid total base assets in vault");
    });

    it("should fail deposit when amount is zero", async () => {
        const [mintSharesPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from(SHARES_MINT_SEED)],
            program.programId
        );

        // Accounts expected by Deposit:
        // 1. signer → User depositing base assets; pays for ATA creation.
        // 2. shares_mint → Vault share mint; must equal vault.shares_mint.
        // 3. shares_ata → Signer's ATA to receive minted shares.
        // 4. base_asset_mint → Underlying asset mint; must equal vault.base_asset_mint.
        // 5. base_asset_ata → Signer's ATA holding the base asset to deposit.
        // 6. vault_base_asset_ata → Vault PDA’s ATA that receives deposited assets.
        // 7. vault → PDA with vault configuration and accounting.
        // 8. token_program → SPL Token or Token-2022 program.
        // 9. associated_token_program → Creates ATAs if missing.
        // 10. system_program → Required for system-level account creation.
        let threw = false;
        try {
            await program.methods
                .deposit(new BN(0))
                .accounts({
                    signer: depositor.publicKey,
                    sharesMint: mintSharesPDA,
                    baseAssetMint: baseAssetMint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                })
                .signers([depositor])
                .rpc();
        } catch (err: any) {
            threw = true;
        }

        assert.isTrue(threw, "deposit with zero amount should fail");
    });

    it("should fail deposit when user has insufficient base asset balance", async () => {
        const [mintSharesPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from(SHARES_MINT_SEED)],
            program.programId
        );

        // Accounts expected by Deposit:
        // 1. signer → User depositing base assets; pays for ATA creation.
        // 2. shares_mint → Vault share mint; must equal vault.shares_mint.
        // 3. shares_ata → Signer's ATA to receive minted shares.
        // 4. base_asset_mint → Underlying asset mint; must equal vault.base_asset_mint.
        // 5. base_asset_ata → Signer's ATA holding the base asset to deposit.
        // 6. vault_base_asset_ata → Vault PDA’s ATA that receives deposited assets.
        // 7. vault → PDA with vault configuration and accounting.
        // 8. token_program → SPL Token or Token-2022 program.
        // 9. associated_token_program → Creates ATAs if missing.
        // 10. system_program → Required for system-level account creation.
        let threw = false;
        try {
            await program.methods
                .deposit(new BN(1_000_000)) // 1 base token
                .accounts({
                    signer: poorUser.publicKey,
                    sharesMint: mintSharesPDA,
                    baseAssetMint: baseAssetMint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                })
                .signers([poorUser])
                .rpc();
        } catch (err: any) {
            threw = true;
        }
        assert.isTrue(threw, "deposit with insufficient base asset balance should fail");
    });

    it("should allocate resources (base_asset) to target ATA", async () => {
        let yieldTargetBaseMintAta = await getOrCreateAssociatedTokenAccount(
            program.provider.connection,
            yieldTarget,
            baseAssetMint,
            yieldTarget.publicKey
        );

        const [vltPDA] = PublicKey.findProgramAddressSync([Buffer.from(VAULT_SEED)], program.programId);
        const vltBefore = await program.account.vault.fetch(vltPDA);

        // Accounts expected by Allocate:
        // 1. signer → Vault admin; only the admin can allocate funds.
        // 2. base_asset_mint → Underlying asset mint; must equal vault.base_asset_mint.
        // 3. vault_base_asset_ata → Vault PDA’s ATA holding base assets to be allocated.
        // 4. vault → PDA with vault configuration and authority seeds.
        // 5. target_ata → Destination ATA that receives the allocated base assets.
        // 6. token_program → SPL Token or Token-2022 program.
        // 7. system_program → Required for system-level account operations.
        try {
            const sig = await program.methods.allocate(new BN(2_000_000)).accounts({
                signer: signer.publicKey,
                baseAssetMint: baseAssetMint,
                targetAta: yieldTargetBaseMintAta.address, // For testing, allocate back to signer's base asset ATA
                tokenProgram: TOKEN_PROGRAM_ID,
            })
                .signers([signer])
                .rpc()

            console.log(
                `Your transaction signature: https://explorer.solana.com/transaction/${sig}?cluster=custom&customUrl=${conn.rpcEndpoint}`
            );
        } catch (err) {
            console.error("Transaction error: ", err);
            throw err;
        }

        await new Promise(resolve => {
            setTimeout(resolve, 200)
        });

        //reload vault account
        const vltAfter = await program.account.vault.fetch(vltPDA);

        //reload target ATA
        const yieldTargetBaseAssetAta = await getOrCreateAssociatedTokenAccount(
            program.provider.connection,
            yieldTarget,
            baseAssetMint,
            yieldTarget.publicKey
        );

        const vltBaseAssetAta = await getOrCreateAssociatedTokenAccount(
            program.provider.connection,
            signer,
            baseAssetMint,
            vltPDA,
            true
        );

        assert(yieldTargetBaseAssetAta.amount == BigInt(2_000_000), "invalid allocated amount to target ATA");
        assert(vltBaseAssetAta.amount == BigInt(8_000_000), "invalid remaining amount in vault's base asset ATA");
        assert(vltAfter.totalBaseAssets.eq(vltBefore.totalBaseAssets), "total_base_assets should not change on allocate");
    });

    it("should fail allocate when caller is not the vault admin", async () => {
        const [vltPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from(VAULT_SEED)],
            program.programId
        );

        const yieldTargetBaseMintAta = await getOrCreateAssociatedTokenAccount(
            conn,
            yieldTarget,
            baseAssetMint,
            yieldTarget.publicKey
        );

        // Accounts expected by Allocate:
        // 1. signer → Vault admin; only the admin can allocate funds.
        // 2. base_asset_mint → Underlying asset mint; must equal vault.base_asset_mint.
        // 3. vault_base_asset_ata → Vault PDA’s ATA holding base assets to be allocated.
        // 4. vault → PDA with vault configuration and authority seeds.
        // 5. target_ata → Destination ATA that receives the allocated base assets.
        // 6. token_program → SPL Token or Token-2022 program.
        // 7. system_program → Required for system-level account operations.
        let threw = false;
        try {
            await program.methods
                .allocate(new BN(1_000_000))
                .accounts({
                    signer: depositor.publicKey, // depositor is not admin
                    baseAssetMint: baseAssetMint,
                    vault: vltPDA,
                    targetAta: yieldTargetBaseMintAta.address,
                    tokenProgram: TOKEN_PROGRAM_ID,
                })
                .signers([depositor])
                .rpc();
        } catch (err: any) {
            threw = true;
        }
        assert.isTrue(threw, "allocate called by non-admin should fail");
    });
})