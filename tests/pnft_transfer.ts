import * as anchor from "@project-serum/anchor";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { buildAndSendTx, createAndFundATA, createFundedWallet, createTokenAuthorizationRules } from "../utils/pnft";
import { PNftTransferClient } from "../utils/PNftTransferClient";

describe("pnft_transfer tests", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);


  const pNftTransferClient = new PNftTransferClient(provider.connection, provider.wallet as anchor.Wallet)

  it('transfers pnft to another account (no ruleset)', async () => {

    const nftOwner = await createFundedWallet(provider);
    const nftReceiver = await createFundedWallet(provider);

    const creators = Array(5)
      .fill(null)
      .map((_) => ({ address: Keypair.generate().publicKey, share: 20 }));

    const { mint, ata } = await createAndFundATA({
      provider: provider,
      owner: nftOwner,
      creators,
      royaltyBps: 1000,
      programmable: true,
    });

    const destAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      nftReceiver,
      mint,
      nftReceiver.publicKey
    );
    const initialReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(initialReceiverBalance.value.uiAmount).to.equal(0)

    const builder = await pNftTransferClient.buildTransferPNFT({
      sourceAta: ata,
      nftMint: mint,
      destAta: destAta.address,
      owner: nftOwner.publicKey,
      receiver: nftReceiver.publicKey
    })
    await buildAndSendTx({
      provider,
      ixs: [await builder.instruction()],
      extraSigners: [nftOwner],
    });

    const newReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(newReceiverBalance.value.uiAmount).to.equal(1)

  });

  it('transfers pnft to another account (1 ruleset)', async () => {
    const nftOwner = await createFundedWallet(provider);

    const name = 'PlayRule123';

    const ruleSetAddr = await createTokenAuthorizationRules(
      provider,
      nftOwner,
      name
    );


    const nftReceiver = await createFundedWallet(provider);

    const creators = Array(5)
      .fill(null)
      .map((_) => ({ address: Keypair.generate().publicKey, share: 20 }));

    const { mint, ata } = await createAndFundATA({
      provider: provider,
      owner: nftOwner,
      creators,
      royaltyBps: 1000,
      programmable: true,
      ruleSetAddr
    });

    const destAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      nftReceiver,
      mint,
      nftReceiver.publicKey
    );
    const initialReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(initialReceiverBalance.value.uiAmount).to.equal(0)

    const builder = await pNftTransferClient.buildTransferPNFT({
      sourceAta: ata,
      nftMint: mint,
      destAta: destAta.address,
      owner: nftOwner.publicKey,
      receiver: nftReceiver.publicKey
    })
    await buildAndSendTx({
      provider,
      ixs: [await builder.instruction()],
      extraSigners: [nftOwner],
    });

    const newReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(newReceiverBalance.value.uiAmount).to.equal(1)

  });
});
