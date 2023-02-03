use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Bad Metadata")]
    BadMetadata,
    #[msg("Bad Ruleset")]
    BadRuleset
}