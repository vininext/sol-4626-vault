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
    const ticker = tickerToBytes16("MYTOKEN");

    const [vltPDA] =
        PublicKey.findProgramAddressSync([Buffer.from(VAULT_SEED), Buffer.from(ticker)], program.programId)
    const [sharesMintPDA] =
        PublicKey.findProgramAddressSync([Buffer.from(SHARES_MINT_SEED), vltPDA.toBuffer()], program.programId)

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
        try {

            const t = Array.from(ticker)
            ticker.slice()
            const sig = await program.methods.initialize(ticker).accounts({
                admin: signer.publicKey,
                vault: vltPDA,
                sharesMint: sharesMintPDA,
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

        const vlt = await program.account.vault.fetch(vltPDA);

        assert(vlt.admin.toString() == signer.publicKey.toString(), "invalid vault admin");
        assert(vlt.baseAssetMint.toString() == baseAssetMint.toString(), "invalid base asset mint");
        assert(vlt.sharesMint.toString() == sharesMintPDA.toString(), "invalid shares mint");
    });


    it("should make first deposit and mint shares", async () => {
        try {
            const sig = await program.methods.deposit(new BN(5_000_000), ticker).accounts({
                signer: depositor.publicKey,
                vault: vltPDA,
                sharesMint: sharesMintPDA,
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
                sharesMintPDA,
                depositor.publicKey
            )

            const mintInfo = await getMint(
                program.provider.connection,
                sharesMintPDA
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
        try {
            const sig = await program.methods
                .deposit(new BN(5_000_000), ticker)
                .accounts({
                    signer: depositor.publicKey,
                    vault: vltPDA,
                    sharesMint: sharesMintPDA,
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
        const mintInfo = await getMint(conn, sharesMintPDA);

        // Depositor's shares ATA
        const depositorMintAta = await getOrCreateAssociatedTokenAccount(
            conn,
            depositor,
            sharesMintPDA,
            depositor.publicKey
        );

        const vlt = await program.account.vault.fetch(vltPDA);

        assert(mintInfo.supply == BigInt(10_000_000), "invalid total shares supply");
        assert(depositorMintAta.amount == BigInt(10_000_000), "invalid total shares in depositor ATA");
        assert(vlt.totalBaseAssets.toNumber() == 10_000_000, "invalid total base assets in vault");
    });

    it("should fail deposit when amount is zero", async () => {
        let threw = false;
        try {
            await program.methods
                .deposit(new BN(0), ticker)
                .accounts({
                    signer: depositor.publicKey,
                    vault: vltPDA,
                    sharesMint: sharesMintPDA,
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
        let threw = false;
        try {
            await program.methods
                .deposit(new BN(1_000_000)) // 1 base token
                .accounts({
                    signer: poorUser.publicKey,
                    vault: vltPDA,
                    sharesMint: sharesMintPDA,
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

        const vltBefore = await program.account.vault.fetch(vltPDA);

        try {
            const sig = await program.methods.allocate(new BN(2_000_000), ticker).accounts({
                signer: signer.publicKey,
                baseAssetMint: baseAssetMint,
                vault: vltPDA,
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

function tickerToBytes16(ticker: string): number[] {
    const bytes = new TextEncoder().encode(ticker.toUpperCase());

    if (bytes.length > 16) {
        throw new Error("Ticker too long");
    }

    const out = new Array(16).fill(0);
    for (let i = 0; i < bytes.length; i++) {
        out[i] = bytes[i];
    }

    return out;
}