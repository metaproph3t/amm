import * as token from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";

describe("amm", async () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  const payer = provider.wallet['payer'];
  anchor.setProvider(provider);
  const amm = anchor.workspace.Amm as Program<Amm>;

  let token0, token1;

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
  });

  it("Can initialize pools", async () => {
    const poolKP = anchor.web3.Keypair.generate();

    await amm.methods.initializePool()
      .accounts({
        pool: poolKP.publicKey,
        token0,
        token1,
      })
      .signers([poolKP])
      .rpc();
  });
});
