import * as token from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import { BN } from "bn.js";
import { assert } from "chai";

describe("amm", async () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  const payer = provider.wallet["payer"];
  anchor.setProvider(provider);
  const amm = anchor.workspace.Amm as Program<Amm>;

  let token0, token1, userToken0Account, userToken1Account;

  before(async () => {
    token0 = await token.createMint(
      connection,
      payer,
      payer.publicKey,
      payer.publicKey,
      9
    );

    token1 = await token.createMint(
      connection,
      payer,
      payer.publicKey,
      payer.publicKey,
      9
    );

    userToken0Account = await token.createAssociatedTokenAccount(
      connection,
      payer,
      token0,
      payer.publicKey
    );

    userToken1Account = await token.createAssociatedTokenAccount(
      connection,
      payer,
      token1,
      payer.publicKey
    );

    await token.mintTo(
      connection,
      payer,
      token0,
      userToken0Account,
      payer,
      1_000 * 1_000_000_000
    );

    await token.mintTo(
      connection,
      payer,
      token1,
      userToken1Account,
      payer,
      1_000 * 1_000_000_000
    );
  });

  it("Works", async () => {
    const [pool] = anchor.web3.PublicKey.findProgramAddressSync([
      anchor.utils.bytes.utf8.encode("pool"),
      token0.toBuffer(),
      token1.toBuffer(),
    ], amm.programId);

    await amm.methods
      .initializePool()
      .accounts({
        pool,
        token0,
        token1,
      })
      .rpc();

    const token0Vault = (await token.getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      token0,
      pool,
      true
    )).address;

    const token1Vault = (await token.getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      token1,
      pool,
      true
    )).address;

    await amm.methods
      .provideLiquidity(new BN(25), new BN(10))
      .accounts({
        pool,
        userToken0Account,
        userToken1Account,
        token0Vault,
        token1Vault,
      })
      .rpc();

    const storedPool = await amm.account.pool.fetch(pool);

    assert.ok(storedPool.token0Reserves.eqn(25));
    assert.ok(storedPool.token1Reserves.eqn(10));

    const preToken0Balance = (
      await token.getAccount(connection, userToken0Account)
    ).amount;
    const preToken1Balance = (
      await token.getAccount(connection, userToken1Account)
    ).amount;

    // 25 * 10 = 250, so k = 250
    // if we add 25 to x, 50 * y = 250 or y = 250 / 50 or y = 5
    // 10 - 5 = 5, so we should get 5 out of the pool
    await amm.methods
      .swap({ token0ToToken1: {} }, new BN(25))
      .accounts({
        pool,
        userToken0Account,
        userToken1Account,
        token0Vault,
        token1Vault,
      })
      .rpc();

    const postToken0Balance = (
      await token.getAccount(connection, userToken0Account)
    ).amount;
    const postToken1Balance = (
      await token.getAccount(connection, userToken1Account)
    ).amount;

    assert.equal(
      postToken0Balance,
      preToken0Balance - BigInt(25),
      "user sent wrong amount"
    );
    assert.equal(
      postToken1Balance,
      preToken1Balance + BigInt(5),
      "user received wrong amount"
    );
  });
});
