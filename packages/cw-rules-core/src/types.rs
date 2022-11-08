use generic_query::GenericQuery;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: this library acting weird on linux and spawning "instantiate", "execute", "query", "reply" of "cw_core" here!!
// pub use voting::status::Status;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug, Copy)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Status {
    /// The proposal is open for voting.
    Open,
    /// The proposal has been rejected.
    Rejected,
    /// The proposal has been passed but has not been executed.
    Passed,
    /// The proposal has been passed and executed.
    Executed,
    /// The proposal has failed or expired and has been closed. A
    /// proposal deposit refund has been issued if applicable.
    Closed,
    // The proposal has failed during execution
    ExecutionFailed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    HasBalanceGte(HasBalanceGte),
    CheckOwnerOfNft(CheckOwnerOfNft),
    CheckProposalStatus(CheckProposalStatus),
    GenericQuery(GenericQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HasBalanceGte {
    pub address: String,
    pub required_balance: cw20::Balance,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CheckOwnerOfNft {
    pub address: String,
    pub nft_address: String,
    pub token_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CheckProposalStatus {
    pub dao_address: String,
    pub proposal_id: u64,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CheckPassedProposals {
    pub dao_address: String,
}
