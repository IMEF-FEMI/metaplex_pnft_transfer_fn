use crate::*;

#[allow(clippy::too_many_arguments)]
pub fn send_pnft<'info>(

    authority_and_owner: &AccountInfo<'info>,
    //(!) payer can't carry data, has to be a normal KP:
    // https://github.com/solana-labs/solana/blob/bda0c606a19ce1cc44b5ab638ff0b993f612e76c/runtime/src/system_instruction_processor.rs#L197
    payer: &AccountInfo<'info>,
    source_ata: &Account<'info, TokenAccount>,
    dest_ata: &Account<'info, TokenAccount>,
    dest_owner: &AccountInfo<'info>,
    nft_mint: &Account<'info, Mint>,
    nft_metadata: &UncheckedAccount<'info>,
    nft_edition: &UncheckedAccount<'info>,
    system_program: &Program<'info, System>,
    token_program: &Program<'info, Token>,
    ata_program: &Program<'info, AssociatedToken>,
    instructions: &UncheckedAccount<'info>,
    owner_token_record: &UncheckedAccount<'info>,
    dest_token_record: &UncheckedAccount<'info>,
    authorization_rules_program: &UncheckedAccount<'info>,
    rules_acc: Option<&AccountInfo<'info>>,
    authorization_data: Option<AuthorizationDataLocal>,
    //if passed, use signed_invoke() instead of invoke()
    // program_signer: Option<&Account<'info, TSwap>>,
) -> Result<()> {
    let mut builder = TransferBuilder::new();

    builder
        .authority(*authority_and_owner.key)
        .token_owner(*authority_and_owner.key)
        .token(source_ata.key())
        .destination_owner(*dest_owner.key)
        .destination(dest_ata.key())
        .mint(nft_mint.key())
        .metadata(nft_metadata.key())
        .edition(nft_edition.key())
        .payer(*payer.key);

    let mut account_infos = vec![
        //   0. `[writable]` Token account
        source_ata.to_account_info(),
        //   1. `[]` Token account owner
        authority_and_owner.to_account_info(),
        //   2. `[writable]` Destination token account
        dest_ata.to_account_info(),
        //   3. `[]` Destination token account owner
        dest_owner.to_account_info(),
        //   4. `[]` Mint of token asset
        nft_mint.to_account_info(),
        //   5. `[writable]` Metadata account
        nft_metadata.to_account_info(),
        //   6. `[optional]` Edition of token asset
        nft_edition.to_account_info(),
        //   7. `[signer] Transfer authority (token or delegate owner)
        authority_and_owner.to_account_info(),
        //   8. `[optional, writable]` Owner record PDA
        //passed in below, if needed
        //   9. `[optional, writable]` Destination record PDA
        //passed in below, if needed
        //   10. `[signer, writable]` Payer
        payer.to_account_info(),
        //   11. `[]` System Program
        system_program.to_account_info(),
        //   12. `[]` Instructions sysvar account
        instructions.to_account_info(),
        //   13. `[]` SPL Token Program
        token_program.to_account_info(),
        //   14. `[]` SPL Associated Token Account program
        ata_program.to_account_info(),
        //   15. `[optional]` Token Authorization Rules Program
        //passed in below, if needed
        //   16. `[optional]` Token Authorization Rules account
        //passed in below, if needed
    ];

    let metadata = assert_decode_metadata(nft_mint, &nft_metadata.to_account_info())?;
    if let Some(standard) = metadata.token_standard {
        if standard == TokenStandard::ProgrammableNonFungible {
            msg!("programmable standard triggered");
            //1. add to builder
            builder
                .owner_token_record(owner_token_record.key())
                .destination_token_record(dest_token_record.key());

            //2. add to accounts (if try to pass these for non-pNFT, will get owner errors, since they don't exist)
            account_infos.push(owner_token_record.to_account_info());
            account_infos.push(dest_token_record.to_account_info());
        }
    }

    //if auth rules passed in, validate & include it in CPI call
    if let Some(config) = metadata.programmable_config {
        match config {
            V1 { rule_set } => {
                if let Some(rule_set) = rule_set {
                    msg!("ruleset triggered");
                    //safe to unwrap here, it's expected
                    let rules_acc = rules_acc.unwrap();

                    //1. validate
                    require!(rule_set == *rules_acc.key, ErrorCode::BadRuleset);

                    //2. add to builder
                    builder.authorization_rules_program(*authorization_rules_program.key);
                    builder.authorization_rules(*rules_acc.key);

                    //3. add to accounts
                    account_infos.push(authorization_rules_program.to_account_info());
                    account_infos.push(rules_acc.to_account_info());
                }
            }
        }
    }

    let transfer_ix = builder
        .build(TransferArgs::V1 {
            amount: 1, //currently 1 only
            authorization_data: authorization_data
                .map(|authorization_data| AuthorizationData::try_from(authorization_data).unwrap()),
        })
        .unwrap()
        .instruction();

    // if let Some(vault) = program_signer {
    //     invoke_signed(&transfer_ix, &account_infos, &[&program_signer.seeds()])?;
    // } else {
    //     invoke(&transfer_ix, &account_infos)?;
    // }
    invoke(&transfer_ix, &account_infos)?;

    Ok(())
}

#[inline(never)]
pub fn assert_decode_metadata<'info>(
    nft_mint: &Account<'info, Mint>,
    metadata_account: &AccountInfo<'info>,
) -> Result<Metadata> {
    let (key, _) = Pubkey::find_program_address(
        &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            nft_mint.key().as_ref(),
        ],
        &mpl_token_metadata::id(),
    );
    if key != *metadata_account.key {
        require!(true, ErrorCode::BadMetadata);
    }
    // Check account owner (redundant because of find_program_address above, but why not).
    if *metadata_account.owner != mpl_token_metadata::id() {
        return Err(error!(ErrorCode::BadMetadata));
    }

    Ok(Metadata::from_account_info(metadata_account)?)
}
