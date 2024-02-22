use thiserror::Error;

use cosmwasm_std::{CoinsError, StdError};

use crate::{authenticator::AuthenticatorError, price::PriceError, spend_limit::SpendLimitError};

/// Never is a placeholder to ensure we don't return any errors
#[derive(Error, Debug)]
pub enum Never {}

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    CoinsError(#[from] CoinsError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid denom: {denom}")]
    InvalidDenom { denom: String },

    #[error("Authenticator error: {0}")]
    AuthenticatorError(#[from] AuthenticatorError),

    #[error("Spend limit error: {0}")]
    SpendLimitError(#[from] SpendLimitError),

    #[error("Price error: {0}")]
    PriceResolutionError(#[from] PriceError),
}
