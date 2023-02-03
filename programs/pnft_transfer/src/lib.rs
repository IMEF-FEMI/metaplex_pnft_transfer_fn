use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_token_auth_rules::payload::{Payload, PayloadType, ProofInfo, SeedsVec};
use mpl_token_metadata::{
    self,
    instruction::{builders::TransferBuilder, InstructionBuilder, TransferArgs},
    processor::AuthorizationData,
    state::{Metadata, ProgrammableConfig::V1, TokenMetadataAccount, TokenStandard},
};
pub mod errors;
pub mod utils;

use errors::ErrorCode;
use utils::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod pnft_transfer {
    use super::*;

    pub fn transfer_pnft<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferPNFT<'info>>,
        authorization_data: Option<AuthorizationDataLocal>,
        rules_acc_present: bool,
    ) -> Result<()> {
        let rem_acc = &mut ctx.remaining_accounts.iter();
        let auth_rules = if rules_acc_present {
            Some(next_account_info(rem_acc)?)
        } else {
            None
        };
        send_pnft(
            &ctx.accounts.owner.to_account_info(),
            &ctx.accounts.owner.to_account_info(),
            &ctx.accounts.src,
            &ctx.accounts.dest,
            &ctx.accounts.receiver.to_account_info(),
            &ctx.accounts.nft_mint,
            &ctx.accounts.nft_metadata,
            &ctx.accounts.edition,
            &ctx.accounts.system_program,
            &ctx.accounts.token_program,
            &ctx.accounts.associated_token_program,
            &ctx.accounts.pnft_shared.instructions,
            &ctx.accounts.owner_token_record,
            &ctx.accounts.dest_token_record,
            &ctx.accounts.pnft_shared.authorization_rules_program,
            auth_rules,
            authorization_data,
            // None,
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct TransferPNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK:
    pub receiver: AccountInfo<'info>,
    #[account(mut)]
    pub src: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub dest: Box<Account<'info, TokenAccount>>,
    pub nft_mint: Box<Account<'info, Mint>>,
    // misc
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // pfnt
    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: assert_decode_metadata + seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            nft_mint.key().as_ref(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub nft_metadata: UncheckedAccount<'info>,
    //note that MASTER EDITION and EDITION share the same seeds, and so it's valid to check them here
    /// CHECK: seeds below
    #[account(
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            nft_mint.key().as_ref(),
            mpl_token_metadata::state::EDITION.as_bytes(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub edition: UncheckedAccount<'info>,
    /// CHECK: seeds below
    #[account(mut,
            seeds=[
                mpl_token_metadata::state::PREFIX.as_bytes(),
                mpl_token_metadata::id().as_ref(),
                nft_mint.key().as_ref(),
                mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
                src.key().as_ref()
            ],
            seeds::program = mpl_token_metadata::id(),
            bump
        )]
    pub owner_token_record: UncheckedAccount<'info>,
    /// CHECK: seeds below
    #[account(mut,
            seeds=[
                mpl_token_metadata::state::PREFIX.as_bytes(),
                mpl_token_metadata::id().as_ref(),
                nft_mint.key().as_ref(),
                mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
                dest.key().as_ref()
            ],
            seeds::program = mpl_token_metadata::id(),
            bump
        )]
    pub dest_token_record: UncheckedAccount<'info>,
    pub pnft_shared: ProgNftShared<'info>,
    //
    // remaining accounts could be passed, in this order:
    // - rules account
    // - mint_whitelist_proof
    // - creator_whitelist_proof
}

#[derive(Accounts)]
pub struct ProgNftShared<'info> {
    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: address below
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: UncheckedAccount<'info>,

    //sysvar ixs don't deserialize in anchor
    /// CHECK: address below
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    /// CHECK: address below
    #[account(address = mpl_token_auth_rules::id())]
    pub authorization_rules_program: UncheckedAccount<'info>,
}
// --------------------------------------- replicating mplex type for anchor IDL export
//have to do this because anchor won't include foreign structs in the IDL

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct AuthorizationDataLocal {
    pub payload: Vec<TaggedPayload>,
}
impl From<AuthorizationDataLocal> for AuthorizationData {
    fn from(val: AuthorizationDataLocal) -> Self {
        let mut p = Payload::new();
        val.payload.into_iter().for_each(|tp| {
            p.insert(tp.name, PayloadType::try_from(tp.payload).unwrap());
        });
        AuthorizationData { payload: p }
    }
}

//Unfortunately anchor doesn't like HashMaps, nor Tuples, so you can't pass in:
// HashMap<String, PayloadType>, nor
// Vec<(String, PayloadTypeLocal)>
// so have to create this stupid temp struct for IDL to serialize correctly
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct TaggedPayload {
    name: String,
    payload: PayloadTypeLocal,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub enum PayloadTypeLocal {
    /// A plain `Pubkey`.
    Pubkey(Pubkey),
    /// PDA derivation seeds.
    Seeds(SeedsVecLocal),
    /// A merkle proof.
    MerkleProof(ProofInfoLocal),
    /// A plain `u64` used for `Amount`.
    Number(u64),
}
impl From<PayloadTypeLocal> for PayloadType {
    fn from(val: PayloadTypeLocal) -> Self {
        match val {
            PayloadTypeLocal::Pubkey(pubkey) => PayloadType::Pubkey(pubkey),
            PayloadTypeLocal::Seeds(seeds) => {
                PayloadType::Seeds(SeedsVec::try_from(seeds).unwrap())
            }
            PayloadTypeLocal::MerkleProof(proof) => {
                PayloadType::MerkleProof(ProofInfo::try_from(proof).unwrap())
            }
            PayloadTypeLocal::Number(number) => PayloadType::Number(number),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct SeedsVecLocal {
    /// The vector of derivation seeds.
    pub seeds: Vec<Vec<u8>>,
}
impl From<SeedsVecLocal> for SeedsVec {
    fn from(val: SeedsVecLocal) -> Self {
        SeedsVec { seeds: val.seeds }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct ProofInfoLocal {
    /// The merkle proof.
    pub proof: Vec<[u8; 32]>,
}
impl From<ProofInfoLocal> for ProofInfo {
    fn from(val: ProofInfoLocal) -> Self {
        ProofInfo { proof: val.proof }
    }
}
