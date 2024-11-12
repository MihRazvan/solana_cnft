import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaCnft } from "../target/types/solana_cnft";
import { Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import { assert } from "chai";

describe("solana_cnft", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaCnft as Program<SolanaCnft>;

  // Your actual cNFT details from Formfunction Candy Machine
  const assetId = new PublicKey("9NB5CaVMRGcZ37aSqux6s5qWiXhqVcewsqVbnSzg1pSf");
  const merkleTree = new PublicKey("FL8e7g71Q3GkAkeen1M1MTawPaf2c6rsXhg2tvXvHVjn");

  // New keypair for fraction mint
  const fractionMint = Keypair.generate();

  // Derive vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), assetId.toBuffer()],
    program.programId
  );

  it("Can lock cNFT", async () => {
    try {
      const tx = await program.methods
        .lockCnft(assetId)
        .accounts({
          owner: provider.wallet.publicKey,
          vault: vaultPda,
          merkleTree: merkleTree,
          fractionMint: fractionMint.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("Lock transaction signature", tx);

      // Fetch and verify the vault
      const vaultAccount = await program.account.vault.fetch(vaultPda);
      assert(vaultAccount.owner.equals(provider.wallet.publicKey));
      assert(vaultAccount.assetId.equals(assetId));
      assert(vaultAccount.merkleTree.equals(merkleTree));
      assert(vaultAccount.fractionMint.equals(fractionMint.publicKey));
      assert(vaultAccount.fractionAmount.eq(new anchor.BN(1000)));

      console.log("Vault Data:", {
        owner: vaultAccount.owner.toString(),
        assetId: vaultAccount.assetId.toString(),
        merkleTree: vaultAccount.merkleTree.toString(),
        fractionMint: vaultAccount.fractionMint.toString(),
        fractionAmount: vaultAccount.fractionAmount.toString(),
        lockedAt: new Date(vaultAccount.lockedAt * 1000).toISOString()
      });
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });

  it("Can unlock cNFT", async () => {
    try {
      const tx = await program.methods
        .unlockCnft()
        .accounts({
          owner: provider.wallet.publicKey,
          vault: vaultPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("Unlock transaction signature", tx);

      // Verify vault is closed
      const vaultAccount = await provider.connection.getAccountInfo(vaultPda);
      assert(!vaultAccount, "Vault should be closed");
      console.log("Vault closed successfully");
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });
});