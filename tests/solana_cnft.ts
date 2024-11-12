import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaCnft } from "../target/types/solana_cnft";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

describe("solana_cnft", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaCnft as Program<SolanaCnft>;

  // Test accounts
  const owner = Keypair.generate();
  let assetId: PublicKey;
  let vaultPda: PublicKey;
  let fractionMint: Keypair;
  let ownerFractionAta: PublicKey;

  before(async () => {
    // Airdrop SOL to owner
    const signature = await provider.connection.requestAirdrop(
      owner.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(signature);

    // Initialize test accounts
    fractionMint = Keypair.generate();
    assetId = Keypair.generate().publicKey; // Simulating a cNFT for now

    // Derive PDAs
    [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), assetId.toBuffer()],
      program.programId
    );
  });

  it("Initializes the program", async () => {
    await program.methods
      .initialize()
      .rpc();
  });

  it("Can lock a cNFT and create fractions", async () => {
    try {
      // Mock Merkle tree data
      const root = Buffer.alloc(32);
      const dataHash = Buffer.alloc(32);
      const creatorHash = Buffer.alloc(32);
      const nonce = new anchor.BN(1);
      const index = 0;

      await program.methods
        .lockCnft(
          assetId,
          Array.from(root),
          Array.from(dataHash),
          Array.from(creatorHash),
          nonce,
          index
        )
        .accounts({
          owner: owner.publicKey,
          vault: vaultPda,
          merkleTree: Keypair.generate().publicKey, // Mock for now
          fractionMint: fractionMint.publicKey,
          ownerFractionAccount: ownerFractionAta,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([owner, fractionMint])
        .rpc();

      // Verify fraction tokens were created
      // Add verification logic here
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });
});