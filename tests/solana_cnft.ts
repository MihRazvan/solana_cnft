import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaCnft } from "../target/types/solana_cnft";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import {
  createCreateTreeInstruction,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import {
  createAccount,
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  MetadataArgs,
  TokenProgramVersion,
  TokenStandard,
} from "@metaplex-foundation/mpl-bubblegum";
import { BN } from "bn.js";
import { assert } from "chai";

describe("solana_cnft", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaCnft as Program<SolanaCnft>;

  // Test accounts
  const payer = provider.wallet;
  const treeCreator = Keypair.generate();
  let merkleTree: PublicKey;
  let treeAuthority: PublicKey;
  let fractionMint: PublicKey;
  let mintAuthority: PublicKey;
  let ownerFractionAccount: PublicKey;

  // Test data
  const maxDepth = 14; // Supports up to 16,384 NFTs
  const maxBufferSize = 64;
  const canopyDepth = 0;

  const metadata: MetadataArgs = {
    name: "Test cNFT",
    symbol: "TEST",
    uri: "https://test.com/nft.json",
    sellerFeeBasisPoints: 500, // 5%
    primarySaleHappened: true,
    isMutable: true,
    editionNonce: null,
    tokenStandard: TokenStandard.NonFungible,
    collection: null,
    uses: null,
    tokenProgramVersion: TokenProgramVersion.Original,
    creators: [
      {
        address: provider.wallet.publicKey,
        verified: false,
        share: 100,
      }
    ]
  };

  before(async () => {
    try {
      // Airdrop to tree creator
      await provider.connection.requestAirdrop(
        treeCreator.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await new Promise(resolve => setTimeout(resolve, 1000)); // Wait for confirmation

      // Create merkle tree
      [merkleTree] = PublicKey.findProgramAddressSync(
        [Buffer.from("merkle-tree")],
        program.programId
      );

      [treeAuthority] = PublicKey.findProgramAddressSync(
        [merkleTree.toBuffer()],
        SPL_ACCOUNT_COMPRESSION_PROGRAM_ID
      );

      const allocTreeIx = await createCreateTreeInstruction(
        {
          merkleTree,
          treeCreator: treeCreator.publicKey,
          payer: payer.publicKey,
          treeAuthority,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
        },
        {
          maxDepth,
          maxBufferSize,
        }
      );

      const tx = new Transaction().add(allocTreeIx);
      await provider.sendAndConfirm(tx, [treeCreator]);

      // Initialize program
      [mintAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from("authority")],
        program.programId
      );

      // Create fraction mint
      const fractionMintKeypair = Keypair.generate();
      fractionMint = fractionMintKeypair.publicKey;

      await program.methods
        .initialize()
        .accounts({
          payer: payer.publicKey,
          fractionMint,
          authority: mintAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      // Get ATA for owner fraction tokens
      ownerFractionAccount = await getAssociatedTokenAddress(
        fractionMint,
        payer.publicKey
      );

    } catch (error) {
      console.error("Setup error:", error);
      throw error;
    }
  });

  describe("Initialization", () => {
    it("Initializes the program state", async () => {
      // Verify fraction mint
      const mintInfo = await provider.connection.getAccountInfo(fractionMint);
      assert(mintInfo !== null, "Fraction mint not created");

      // Verify mint authority
      const mintAuthInfo = await provider.connection.getAccountInfo(mintAuthority);
      assert(mintAuthInfo !== null, "Mint authority not created");
    });

    it("Verifies merkle tree setup", async () => {
      const treeInfo = await provider.connection.getAccountInfo(merkleTree);
      assert(treeInfo !== null, "Merkle tree not created");
      assert(treeInfo.owner.equals(SPL_ACCOUNT_COMPRESSION_PROGRAM_ID), "Invalid tree owner");
    });
  });

  describe("NFT Operations", () => {
    it("Can lock cNFT", async () => {
      try {
        // Test data
        const nonce = new BN(0);
        const index = 0;

        // Get PDA for vault
        const [vault] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("vault"),
            merkleTree.toBuffer(),
            nonce.toArrayLike(Buffer, 'le', 8)
          ],
          program.programId
        );

        // Mock proof data
        const root = Buffer.alloc(32);
        const dataHash = Buffer.alloc(32);
        const creatorHash = Buffer.alloc(32);
        const proofNodes = Array(maxDepth).fill(Buffer.alloc(32));

        const tx = await program.methods
          .lockCnft(
            metadata,
            Array.from(root),
            Array.from(dataHash),
            Array.from(creatorHash),
            nonce,
            index
          )
          .accounts({
            owner: payer.publicKey,
            vault,
            treeAuthority,
            merkleTree,
            fractionMint,
            ownerFractionAccount,
            authority: mintAuthority,
            bubblegumProgram: new PublicKey("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY"),
            logWrapper: SPL_NOOP_PROGRAM_ID,
            compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
          })
          .remainingAccounts(
            proofNodes.map(node => ({
              pubkey: new PublicKey(node),
              isWritable: false,
              isSigner: false
            }))
          );

        const txHash = await tx.rpc();
        console.log("Lock transaction:", txHash);

        // Verify vault state
        const vaultAccount = await program.account.vault.fetch(vault);
        assert(vaultAccount.owner.equals(payer.publicKey), "Invalid vault owner");
        assert(vaultAccount.merkleTree.equals(merkleTree), "Invalid merkle tree reference");
        assert.deepEqual(Array.from(vaultAccount.root), Array.from(root), "Invalid root hash");

        // Verify fraction tokens
        const tokenBalance = await provider.connection.getTokenAccountBalance(ownerFractionAccount);
        assert.equal(tokenBalance.value.uiAmount, 1000, "Invalid fraction token amount");

      } catch (error) {
        console.error("Lock error:", error);
        throw error;
      }
    });

    it("Can unlock cNFT", async () => {
      try {
        // Get vault account
        const nonce = new BN(0);
        const [vault] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("vault"),
            merkleTree.toBuffer(),
            nonce.toArrayLike(Buffer, 'le', 8)
          ],
          program.programId
        );

        // Mock proof data 
        const proofNodes = Array(maxDepth).fill(Buffer.alloc(32));

        const tx = await program.methods
          .unlockCnft()
          .accounts({
            owner: payer.publicKey,
            vault,
            treeAuthority,
            merkleTree,
            fractionMint,
            ownerFractionAccount,
            bubblegumProgram: new PublicKey("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY"),
            logWrapper: SPL_NOOP_PROGRAM_ID,
            compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .remainingAccounts(
            proofNodes.map(node => ({
              pubkey: new PublicKey(node),
              isWritable: false,
              isSigner: false
            }))
          );

        const txHash = await tx.rpc();
        console.log("Unlock transaction:", txHash);

        // Verify vault is closed
        const vaultAccount = await provider.connection.getAccountInfo(vault);
        assert(!vaultAccount, "Vault should be closed");

        // Verify fraction tokens are burned
        const tokenBalance = await provider.connection.getTokenAccountBalance(ownerFractionAccount);
        assert.equal(tokenBalance.value.uiAmount, 0, "Fraction tokens not burned");

      } catch (error) {
        console.error("Unlock error:", error);
        throw error;
      }
    });
  });

  describe("Error Cases", () => {
    it("Cannot unlock without owning all fractions", async () => {
      // TODO: Implement test for attempting unlock without full fraction balance
    });

    it("Cannot lock same cNFT twice", async () => {
      // TODO: Implement test for attempting to lock already locked cNFT
    });

    it("Fails with invalid proofs", async () => {
      // TODO: Implement test for invalid merkle proof handling
    });
  });
});