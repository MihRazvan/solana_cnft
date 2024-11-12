import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaCnft } from "../target/types/solana_cnft";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY
} from '@solana/web3.js';
import { assert } from "chai";

describe("solana_cnft", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaCnft as Program<SolanaCnft>;

  // Your existing cNFT details
  const assetId = new PublicKey("9NB5CaVMRGcZ37aSqux6s5qWiXhqVcewsqVbnSzg1pSf");
  const merkleTree = new PublicKey("FL8e7g71Q3GkAkeen1M1MTawPaf2c6rsXhg2tvXvHVjn");

  // Generate fractionMint keypair early
  const fractionMint = anchor.web3.Keypair.generate();

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
    try {
      const tx = await program.methods
        .initialize()
        .accounts({
          payer: provider.wallet.publicKey,
          fractionMint: fractionMint.publicKey,
          authority,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([fractionMint])
        .rpc();

      console.log("Initialize transaction signature", tx);
    } catch (error) {
      console.error("Initialize error:", error);
      throw error;
    }
  });

  it("Can lock cNFT", async () => {
    try {
      // Create ATA for owner
      const ownerAta = await anchor.utils.token.associatedAddress({
        mint: fractionMint.publicKey,
        owner: provider.wallet.publicKey,
      });

      const tx = await program.methods
        .lockCnft(assetId)
        .accounts({
          owner: provider.wallet.publicKey,
          vault: vaultPda,
          merkleTree: merkleTree,
          fractionMint: fractionMint.publicKey,
          ownerFractionAccount: ownerAta,
          authority,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
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

      console.log("Vault Data:", {
        owner: vaultAccount.owner.toString(),
        assetId: vaultAccount.assetId.toString(),
        merkleTree: vaultAccount.merkleTree.toString(),
        lockedAt: new Date(vaultAccount.lockedAt * 1000).toISOString()
      });
    } catch (error) {
      console.error("Lock error:", error);
      throw error;
    }
  });

  it("Can verify fraction token balance after locking", async () => {
    const ownerAta = await anchor.utils.token.associatedAddress({
      mint: fractionMint.publicKey,
      owner: provider.wallet.publicKey,
    });

    const balance = await provider.connection.getTokenAccountBalance(ownerAta);
    assert.equal(balance.value.amount, "1000");
    assert.equal(balance.value.decimals, 0);
  });

  it("Can unlock cNFT", async () => {
    try {
      // Get owner's ATA
      const ownerAta = await anchor.utils.token.associatedAddress({
        mint: fractionMint.publicKey,
        owner: provider.wallet.publicKey,
      });

      const tx = await program.methods
        .unlockCnft()
        .accounts({
          owner: provider.wallet.publicKey,
          vault: vaultPda,
          fractionMint: fractionMint.publicKey,
          ownerFractionAccount: ownerAta,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("Unlock transaction signature", tx);

      // Verify vault is closed
      const vaultAccount = await provider.connection.getAccountInfo(vaultPda);
      assert(!vaultAccount, "Vault should be closed");
      console.log("Vault closed successfully");
    } catch (error) {
      console.error("Unlock error:", error);
      throw error;
    }
  });
});