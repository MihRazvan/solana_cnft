import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaCnft } from "../target/types/solana_cnft";
import { PublicKey, SystemProgram } from '@solana/web3.js';
import { assert } from "chai";

describe("solana_cnft", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaCnft as Program<SolanaCnft>;

  // Your existing cNFT details
  const existingCnftMint = new PublicKey("FL8e7g71Q3GkAkeen1M1MTawPaf2c6rsXhg2tvXvHVjn"); // Replace with your cNFT mint
  const merkleTree = new PublicKey("Hs5BNJJZzQ8gzyx4ng5eH7GJrwYKYAV1jeY9GjvSu38n"); // Replace with your merkle tree

  // Derive vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), existingCnftMint.toBuffer()],
    program.programId
  );

  it("Can lock cNFT", async () => {
    try {
      const tx = await program.methods
        .lockCnft(existingCnftMint)
        .accounts({
          owner: provider.wallet.publicKey,
          vault: vaultPda,
          merkleTree: merkleTree,
          treeConfig: merkleTree, // We might need to derive this properly
          logWrapper: new PublicKey("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV"),
          compressionProgram: new PublicKey("cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK"),
          bubblegumProgram: new PublicKey("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY"),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("Lock transaction signature", tx);

      // Fetch the vault to verify
      const vault = await program.account.vault.fetch(vaultPda);
      assert(vault.owner.equals(provider.wallet.publicKey));
      assert(vault.assetId.equals(existingCnftMint));
      console.log("Vault created successfully");
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