import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clmm } from "../target/types/clmm";
import { PublicKey, Keypair, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import { expect } from "chai";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";

describe("clmm", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.clmm as Program<Clmm>;
  const provider = anchor.getProvider();

  // Test accounts
  let user: Keypair;
  let mintA: PublicKey;
  let mintB: PublicKey;
  let userTokenAccountA: PublicKey;
  let userTokenAccountB: PublicKey;
  let pool: PublicKey;
  let lpMint: PublicKey;
  let vaultA: PublicKey;
  let vaultB: PublicKey;
  let userLpAccount: PublicKey;
  let tickLower: PublicKey;
  let tickUpper: PublicKey;

  const TICK_SPACING = 1;
  const INITIAL_PRICE = 1000000; // 1.0 in price units
  const SEED = 12345;

  before(async () => {
    // Create test user
    user = Keypair.generate();
    
    // Airdrop SOL to user
    const signature = await provider.connection.requestAirdrop(user.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(signature);

    // Create test tokens
    mintA = await createMint(
      provider.connection,
      user,
      user.publicKey,
      user.publicKey,
      6
    );

    mintB = await createMint(
      provider.connection,
      user,
      user.publicKey,
      user.publicKey,
      6
    );

    // Create user token accounts
    userTokenAccountA = await createAccount(
      provider.connection,
      user,
      mintA,
      user.publicKey
    );

    userTokenAccountB = await createAccount(
      provider.connection,
      user,
      mintB,
      user.publicKey
    );

    // Mint some tokens to user
    await mintTo(
      provider.connection,
      user,
      mintA,
      userTokenAccountA,
      user,
      1000000000 // 1000 tokens
    );

    await mintTo(
      provider.connection,
      user,
      mintB,
      userTokenAccountB,
      user,
      1000000000 // 1000 tokens
    );

    // Derive pool and related accounts
    const [poolKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), new anchor.BN(SEED).toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    pool = poolKey;

    const [lpMintKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), pool.toBuffer()],
      program.programId
    );
    lpMint = lpMintKey;

    const [vaultAKey] = PublicKey.findProgramAddressSync(
      [pool.toBuffer(), mintA.toBuffer(), Buffer.from("vault")],
      program.programId
    );
    vaultA = vaultAKey;

    const [vaultBKey] = PublicKey.findProgramAddressSync(
      [pool.toBuffer(), mintB.toBuffer(), Buffer.from("vault")],
      program.programId
    );
    vaultB = vaultBKey;

    const [userLpAccountKey] = PublicKey.findProgramAddressSync(
      [user.publicKey.toBuffer(), lpMint.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    userLpAccount = userLpAccountKey;

    // Derive tick accounts
    const [tickLowerKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("tick"), pool.toBuffer(), new anchor.BN(-100).toArrayLike(Buffer, "le", 4)],
      program.programId
    );
    tickLower = tickLowerKey;

    const [tickUpperKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("tick"), pool.toBuffer(), new anchor.BN(100).toArrayLike(Buffer, "le", 4)],
      program.programId
    );
    tickUpper = tickUpperKey;
  });

  it("Initialize pool", async () => {
    try {
      const tx = await program.methods
        .initPool(new anchor.BN(SEED), new anchor.BN(INITIAL_PRICE))
        .accountsStrict({
          signer: user.publicKey,
          minta: mintA,
          mintb: mintB,
          lpMint: lpMint,
          vaulta: vaultA,
          config: pool,
          vaultB: vaultB,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([user])
        .rpc();

      console.log("Pool initialized with signature:", tx);
      
      // Verify pool was created
      const poolAccount = await program.account.pool.fetch(pool);
      expect(poolAccount.minta.toString()).to.equal(mintA.toString());
      expect(poolAccount.mintb.toString()).to.equal(mintB.toString());
      expect(poolAccount.seed.toNumber()).to.equal(SEED);
    } catch (error) {
      console.error("Error initializing pool:", error);
      throw error;
    }
  });

  // it("Initialize tick accounts", async () => {
  //   try {
  //     // Initialize lower tick
  //     const tx1 = await program.methods
  //       .initTick()
  //       .accountsStrict({
  //         signer: user.publicKey,
  //         config: pool,
  //         tick: tickLower,
  //         systemProgram: SystemProgram.programId,
  //       })
  //       .signers([user])
  //       .rpc();

  //     console.log("Lower tick initialized with signature:", tx1);

  //     // Initialize upper tick
  //     const tx2 = await program.methods
  //       .initTick()
  //       .accountsStrict({
  //         signer: user.publicKey,
  //         config: pool,
  //         tick: tickUpper,
  //         systemProgram: SystemProgram.programId,
  //       })
  //       .signers([user])
  //       .rpc();

  //     console.log("Upper tick initialized with signature:", tx2);

  //     // Verify ticks were created
  //     const lowerTickAccount = await program.account.tick.fetch(tickLower);
  //     const upperTickAccount = await program.account.tick.fetch(tickUpper);
      
  //     expect(lowerTickAccount.index).to.equal(-100);
  //     expect(upperTickAccount.index).to.equal(100);
  //   } catch (error) {
  //     console.error("Error initializing ticks:", error);
  //     throw error;
  //   }
  // });

  // it("Add liquidity", async () => {
  //   try {
  //     const liquidity = new anchor.BN(1000000); // 1M liquidity units
  //     const tickLowerIndex = -100;
  //     const tickUpperIndex = 100;

  //     const tx = await program.methods
  //       .addLiquidity(
  //         tickLowerIndex,
  //         tickUpperIndex,
  //         liquidity
  //       )
  //       .accountsStrict({
  //         signer: user.publicKey,
  //         minta: mintA,
  //         mintb: mintB,
  //         lpMint: lpMint,
  //         usertokenAccountA: userTokenAccountA,
  //         usertokenAccountB: userTokenAccountB,
  //         userLpAccount: userLpAccount,
  //         vaulta: vaultA,
  //         config: pool,
  //         vaultB: vaultB,
  //         uppertick: tickUpper,
  //         lowertick: tickLower,
  //         systemProgram: SystemProgram.programId,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //       })
  //       .signers([user])
  //       .rpc();

  //     console.log("Liquidity added with signature:", tx);

  //     // Verify LP tokens were minted
  //     const lpAccount = await getAccount(provider.connection, userLpAccount);
  //     expect(Number(lpAccount.amount)).to.be.greaterThan(0);

  //     // Verify pool liquidity increased
  //     const poolAccount = await program.account.pool.fetch(pool);
  //     expect(poolAccount.activeLiqiudity.toNumber()).to.be.greaterThan(0);
  //   } catch (error) {
  //     console.error("Error adding liquidity:", error);
  //     throw error;
  //   }
  // });

  // // it("Swap tokens", async () => {
  // //   try {
  // //     const amountIn = new anchor.BN(100000); // 0.1 tokens
  // //     const aToB = true; // Swap from token A to token B
  // //     const sqrtPriceLimit = null; // No price limit
  // //     const minAmountOut = new anchor.BN(90000); // 0.09 tokens minimum output

  // //     // Get initial balances
  // //     const initialBalanceA = await getAccount(provider.connection, userTokenAccountA);
  // //     const initialBalanceB = await getAccount(provider.connection, userTokenAccountB);

  // //     const tx = await program.methods
  // //       .swap(
  // //         amountIn,
  // //         sqrtPriceLimit,
  // //         minAmountOut,
  // //         aToB
  // //       )
  // //       .accountsStrict({
  // //         useraccount: user.publicKey,
  // //         minta: mintA,
  // //         mintb: mintB,
  // //         usertokenAccountA: userTokenAccountA,
  // //         usertokenAccountB: userTokenAccountB,
  // //         vaulta: vaultA,
  // //         config: pool,
  // //         vaultB: vaultB,
  // //         tokenProgram: TOKEN_PROGRAM_ID,
  // //         systemProgram:SYSTEM_PROGRAM_ID,
  // //         lowertick:
  // //       })
  // //       .remainingAccounts([
  // //         { pubkey: tickLower, isSigner: false, isWritable: true },
  // //         { pubkey: tickUpper, isSigner: false, isWritable: true },
  // //       ])
  // //       .signers([user])
  // //       .rpc();

  // //     console.log("Swap executed with signature:", tx);

  // //     // Verify balances changed
  // //     const finalBalanceA = await getAccount(provider.connection, userTokenAccountA);
  // //     const finalBalanceB = await getAccount(provider.connection, userTokenAccountB);

  // //     expect(Number(finalBalanceA.amount)).to.be.lessThan(Number(initialBalanceA.amount));
  // //     expect(Number(finalBalanceB.amount)).to.be.greaterThan(Number(initialBalanceB.amount));
  // //   } catch (error) {
  // //     console.error("Error executing swap:", error);
  // //     throw error;
  // //   }
  // // });

  // it("Withdraw liquidity", async () => {
  //   try {
  //     const liquidityToRemove = new anchor.BN(500000); // Remove half the liquidity
  //     const tickLowerIndex = -100;
  //     const tickUpperIndex = 100;

  //     // Get initial LP balance
  //     const initialLpBalance = await getAccount(provider.connection, userLpAccount);

  //     const tx = await program.methods
  //       .withdrawLiquidity(
  //         tickLowerIndex,
  //         tickUpperIndex,
  //         liquidityToRemove
  //       )
  //       .accountsStrict
  //       ({
  //         signer: user.publicKey,
  //         minta: mintA,
  //         mintb: mintB,
  //         lpMint: lpMint,
  //         usertokenAccountA: userTokenAccountA,
  //         usertokenAccountB: userTokenAccountB,
  //         userLpAccount: userLpAccount,
  //         vaulta: vaultA,
  //         config: pool,
  //         vaultB: vaultB,
  //         uppertick: tickUpper,
  //         lowertick: tickLower,
  //         systemProgram: SystemProgram.programId,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //       })
  //       .signers([user])
  //       .rpc();

  //     console.log("Liquidity withdrawn with signature:", tx);

  //     // Verify LP tokens were burned
  //     const finalLpBalance = await getAccount(provider.connection, userLpAccount);
  //     expect(Number(finalLpBalance.amount)).to.be.lessThan(Number(initialLpBalance.amount));

  //     // Verify pool liquidity decreased
  //     const poolAccount = await program.account.pool.fetch(pool);
  //     expect(poolAccount.activeLiqiudity.toNumber()).to.be.greaterThan(0);
  //   } catch (error) {
  //     console.error("Error withdrawing liquidity:", error);
  //     throw error;
  //   }
  // });

  // it("Should fail with invalid tick range", async () => {
  //   try {
  //     const liquidity = new anchor.BN(1000000);
  //     const tickLowerIndex = 100; // Upper tick
  //     const tickUpperIndex = -100; // Lower tick (invalid order)

  //     await program.methods
  //       .addLiquidity(
  //         tickLowerIndex,
  //         tickUpperIndex,
  //         liquidity
  //       )
  //       .accountsStrict({
  //         signer: user.publicKey,
  //         minta: mintA,
  //         mintb: mintB,
  //         lpMint: lpMint,
  //         usertokenAccountA: userTokenAccountA,
  //         usertokenAccountB: userTokenAccountB,
  //         userLpAccount: userLpAccount,
  //         vaulta: vaultA,
  //         config: pool,
  //         vaultB: vaultB,
  //         uppertick: tickUpper,
  //         lowertick: tickLower,
  //         systemProgram: SystemProgram.programId,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //       })
  //       .signers([user])
  //       .rpc();

  //     // Should not reach here
  //     expect.fail("Should have thrown an error");
  //   } catch (error) {
  //     console.log("Expected error caught:", error.message);
  //     expect(error.message).to.include("error");
  //   }
  // });

  // it("Should fail with zero amount swap", async () => {
  //   try {
  //     const amountIn = new anchor.BN(0); // Zero amount
  //     const aToB = true;

  //     await program.methods
  //       .swap(
  //         amountIn,
  //         null,
  //         null,
  //         aToB
  //       )
  //       .accountsStrict({
  //         useraccount: user.publicKey,
  //         minta: mintA,
  //         mintb: mintB,
  //         usertokenAccountA: userTokenAccountA,
  //         usertokenAccountB: userTokenAccountB,
  //         vaulta: vaultA,
  //         config: pool,
  //         vaultB: vaultB,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       })
  //       .remainingAccounts([
  //         { pubkey: tickLower, isSigner: false, isWritable: true },
  //         { pubkey: tickUpper, isSigner: false, isWritable: true },
  //       ])
  //       .signers([user])
  //       .rpc();

  //     // Should not reach here
  //     expect.fail("Should have thrown an error");
  //   } catch (error) {
  //     console.log("Expected error caught:", error.message);
  //     expect(error.message).to.include("error");
  //   }
  // });
});
