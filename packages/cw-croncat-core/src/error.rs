use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum CoreError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Boundary is not in valid format")]
    InvalidBoundary {},

    #[error("No coin balance found")]
    EmptyBalance {},

    #[error("Not enough cw20 balance of {addr}, need {lack} more")]
    NotEnoughCw20 { addr: String, lack: Uint128 },

    #[error("invalid cosmwasm message")]
    InvalidWasmMsg {},
}
