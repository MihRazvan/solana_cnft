import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaCnft } from "../target/types/solana_cnft";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress
} from '@solana/spl-token';
import { assert } from "chai";

describe("solana_cnft", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaCnft as Program<SolanaCnft>;

  // Our existing cNFT details
  const assetId = new PublicKey("9NB5CaVMRGcZ37aSqux6s5qWiXhqVcewsqVbnSzg1pSf");
  const merkleTree = new PublicKey("FL8e7g71Q3GkAkeen1M1MTawPaf2c6rsXhg2tvXvHVjn");

  // New fraction mint
  const fractionMint = Keypair.generate();

  // Find PDAs
  const [authority] = PublicKey.findProgramAddressSync(
    [Buffer.from("authority")],
    program.programId
  );

  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), assetId.toBuffer()],
    program.programId
  );

  it("Initialize fraction mint", async () => {
    // Get user's ATA for fraction tokens
    const ownerAta = await getAssociatedTokenAddress(
      fractionMint.publicKey,
      provider.wallet.publicKey
    );

    const tx = await program.methods
      .initialize()
      .accounts({
        payer: provider.wallet.publicKey,
        fractionMint: fractionMint.publicKey,
        authority,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([fractionMint])
      .rpc();

    console.log("Initialize transaction signature", tx);
  });

  it("Can lock cNFT and receive fractions", async () => {
    // Get user's ATA for fraction tokens
    const ownerAta = await getAssociatedTokenAddress(
      fractionMint.publicKey,
      provider.wallet.publicKey
    );

    const tx = await program.methods
      .lockCnft(assetId)
      .accounts({
        owner: provider.wallet.publicKey,
        vault: vaultPda,
        merkleTree: merkleTree,
        fractionMint: fractionMint.publicKey,
        ownerFractionAccount: ownerAta,
        authority,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Lock transaction signature", tx);

    // Fetch and verify the vault
    const vaultAccount = await program.account.vault.fetch(vaultPda);
    assert(vaultAccount.owner.equals(provider.wallet.publicKey));
    assert(vaultAccount.assetId.equals(assetId));
    assert(vaultAccount.merkleTree.equals(merkleTree));

    // Verify fraction token balance
    const balance = (await provider.connection.getTokenAccountBalance(ownerAta)).value;
    assert.equal(balance.amount, "1000");
    assert.equal(balance.decimals, 0);

    console.log("Vault Data:", {
      owner: vaultAccount.owner.toString(),
      assetId: vaultAccount.assetId.toString(),
      merkleTree: vaultAccount.merkleTree.toString(),
      lockedAt: new Date(vaultAccount.lockedAt * 1000).toISOString()
    });
  });

  it("Can unlock cNFT and burn fractions", async () => {
    // Get user's ATA for fraction tokens
    const ownerAta = await getAssociatedTokenAddress(
      fractionMint.publicKey,
      provider.wallet.publicKey
    );

    const tx = await program.methods
      .unlockCnft()
      .accounts({
        owner: provider.wallet.publicKey,
        vault: vaultPda,
        fractionMint: fractionMint.publicKey,
        ownerFractionAccount: ownerAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Unlock transaction signature", tx);

    // Verify vault is closed
    const vaultAccount = await provider.connection.getAccountInfo(vaultPda);
    assert(!vaultAccount, "Vault should be closed");

    // Verify fractions are burned
    const balance = (await provider.connection.getTokenAccountBalance(ownerAta)).value;
    assert.equal(balance.amount, "0");

    console.log("Vault closed successfully");
  });
});