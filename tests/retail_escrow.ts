import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { RetailEscrow } from "../target/types/retail_escrow";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo } from "@solana/spl-token";
import { assert } from "chai";

describe("retail_escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.RetailEscrow as Program<RetailEscrow>;

  let mint: PublicKey;
  let buyerTokenAccount: PublicKey;
  let escrowTokenAccount: PublicKey;
  let buyer: Keypair;
  let retailer: Keypair;

  before(async () => {
    // Create a new mint and buyer

    const payer = Keypair.generate();
    await provider.connection.requestAirdrop(payer.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(payer.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL));
    mint = await createMint(provider.connection, payer, payer.publicKey, null, 6);
    buyer = Keypair.generate();
    retailer = Keypair.generate();

    // Airdrop SOL to buyer
    await provider.connection.requestAirdrop(buyer.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);

    // Create token accounts
    buyerTokenAccount = await createAccount(provider.connection, buyer, mint, buyer.publicKey);
    escrowTokenAccount = await createAccount(provider.connection, payer, mint, program.programId);

    // Mint tokens to buyer
    await mintTo(provider.connection, payer, mint, buyerTokenAccount, payer, 1000000);
  }); 

  it("Initializes escrow", async () => {
    const escrowId = new anchor.BN(1);
    const amount = new anchor.BN(100000);
    const buyerBalance = await provider.connection.getTokenAccountBalance(buyerTokenAccount);
    console.log("buyerBalanc:::", buyerBalance);

    const [escrowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), escrowId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    await program.methods
      .initializeEscrow(escrowId, amount, retailer.publicKey)
      .accountsStrict({
        buyer: buyer.publicKey,
        escrow: escrowPda,
        buyerTokenAccount: buyerTokenAccount,
        escrowTokenAccount: escrowTokenAccount,
        retailer: retailer.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([buyer])
      .rpc();

    // Fetch the created escrow account
    const escrowAccount = await program.account.escrow.fetch(escrowPda);

    // Assert the escrow account data
    assert.ok(escrowAccount.buyer.equals(buyer.publicKey), "Buyer pubkey does not match");
    assert.ok(escrowAccount.retailer.equals(retailer.publicKey), "Retailer pubkey does not match");
    assert.ok(escrowAccount.escrowId.eq(escrowId), "Escrow ID does not match");
    assert.ok(escrowAccount.amount.eq(amount), "Amount does not match");
    assert.equal(escrowAccount.state, { awaitingDelivery: {} }, "Incorrect initial state");

    // Check if the tokens were transferred to the escrow account
    const escrowBalance = await provider.connection.getTokenAccountBalance(escrowTokenAccount);
    assert.equal(escrowBalance.value.amount, amount.toString(), "Escrow token account balance is incorrect");
  });

  it("Fails to initialize escrow with insufficient funds", async () => {
    const escrowId = new anchor.BN(2);
    const amount = new anchor.BN(2000000); // More than the buyer has

    const [escrowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), escrowId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    try {
      await program.methods
        .initializeEscrow(escrowId, amount, retailer.publicKey)
        .accountsStrict({
          buyer: buyer.publicKey,
          escrow: escrowPda,
          buyerTokenAccount: buyerTokenAccount,
          escrowTokenAccount: escrowTokenAccount,
          retailer: retailer.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([buyer])
        .rpc();
      assert.fail("Expected an error, but the transaction succeeded");
    } catch (error) {
      assert.include(error.message, "0x1"); // Insufficient funds error
    }
  });
});