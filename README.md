
# Solana cNFT Fractionalization Program

## Overview
A Solana program that enables users to lock their compressed NFTs (cNFTs) and receive fungible SPL tokens in return. The program implements a 1:1000 fractionalization mechanism, where each locked cNFT generates 1000 fungible tokens that can later be burned to retrieve the original cNFT.

## Initial Project Setup

### Prerequisites
- Solana CLI (1.16.17 or later)
- Anchor (0.30.1)
- Rust (1.82.0)
- Node.js and yarn

### Environment Setup
1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd solana_cnft
   ```

2. Install dependencies:
   ```bash
   yarn install
   ```

3. Build the program:
   ```bash
   anchor build
   ```

## Integration with Formfunction Candy Machine
The program is designed to work with cNFTs minted through Formfunction's Candy Machine (a fork of Metaplex Candy Machine v2). In our testing environment, we used a cNFT with:

- **Asset ID**: 9NB5CaVMRGcZ37aSqux6s5qWiXhqVcewsqVbnSzg1pSf
- **Merkle Tree**: FL8e7g71Q3GkAkeen1M1MTawPaf2c6rsXhg2tvXvHVjn

## Technical Details

### Architecture
The program consists of three main components:
- **Vault**: Stores locked cNFT information
- **Fraction Mint**: Global SPL token mint for fractional tokens
- **Authority PDA**: Controls token minting and burning

### Account Structure
```rust
#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub asset_id: Pubkey,
    pub merkle_tree: Pubkey,
    pub locked_at: i64,
}
```

### Key PDAs and Seeds
- **Vault**: ["vault", asset_id]
- **Authority**: ["authority"]

### Instructions
- **initialize**: Creates the global fraction mint
- **lock_cnft**: Locks a cNFT and mints 1000 fraction tokens
- **unlock_cnft**: Burns fraction tokens and releases the cNFT

### Security Features
- Ownership verification before unlocking
- PDA-based authority for token minting
- Full token burn requirement for unlocking

## Testing Instructions

### Local Testing Environment
Configure your local Solana environment for devnet:
```bash
solana config set --url devnet
```

Ensure you have enough devnet SOL:
```bash
solana airdrop 2
```

### Running Tests
Build the program:
```bash
anchor build
```

Run the test suite:
```bash
anchor test
```

### Expected Test Output
The tests verify:
- Initialization of fraction mint
- Locking of cNFT
- Correct minting of 1000 fraction tokens
- Successful unlocking and token burning

Sample test output:
```plaintext
Initialize transaction signature [signature]
Vault Data: {
  owner: '7Shhbu7KcCjGx1eLhdnMBKKBqKsbtUh9CoGBsTSmKYCS',
  assetId: '9NB5CaVMRGcZ37aSqux6s5qWiXhqVcewsqVbnSzg1pSf',
  merkleTree: 'FL8e7g71Q3GkAkeen1M1MTawPaf2c6rsXhg2tvXvHVjn',
  lockedAt: [timestamp]
}
4 passing
```

### Verifying Fractionalization
Monitor token balances using:
```bash
spl-token accounts
```

Check vault status:
```bash
anchor run get-vault [vault-address]
```

## Program Dependencies
```toml
[dependencies]
anchor-lang = { version = "0.30.1", features = ["init-if-needed"] }
anchor-spl = { version = "0.30.1", features = ["token", "associated_token"] }
```

## Design Decisions
- **Single Global Fraction Mint**: Instead of creating a new token type for each locked cNFT, we use a single fraction token to simplify tracking and improve efficiency.
- **Fixed Fraction Amount**: Each cNFT is fractionalized into exactly 1000 tokens for consistency and simplicity.
- **Stateless Design**: The program maintains minimal state, primarily using the vault to track locked cNFTs.

## Future Improvements
- Configurable fraction amounts
- Metadata for fraction tokens
- Transfer restrictions
- Integration with DEX for fraction trading

## License
ISC

## Discussion Points for Further Development

### Token Strategy Considerations
1. **Token Uniqueness Strategy**: Currently, the program uses a global fraction token mint for all locked cNFTs. An alternative approach would be to create unique token mints for each locked cNFT. Each approach has different implications for:
   - Token tracking and management
   - Trading on DEXes
   - Gas efficiency
   - User experience
Would the current global token approach align with your intended use case, or should we implement unique tokens per cNFT?

### Fractionalization Parameters
2. **Dynamic Token Supply**: The program currently implements a fixed 1:1000 ratio for fractionalization. We could make this dynamic based on various factors:
   - cNFT metadata attributes
   - Collection rarity
   - Market factors
   - User-specified parameters
What parameters would be most relevant for determining the fraction amount in your use case?

### Feature Extensions
3. **Potential Enhancements**: Several extensions could add value to the current implementation:
   - Fraction token metadata reflecting the original cNFT
   - Integration with existing DEX protocols for fraction trading
   - Governance mechanisms for fraction holders
   - Partial unlocking mechanisms
   - Price feeds for automatic valuation
Which of these features would be most beneficial for your intended use case?
