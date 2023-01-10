use cosmwasm_schema::{cw_serde, QueryResponses};
use mod_sdk::types::QueryResponse;

use crate::types::CheckProposalStatus;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Query proposal status and compare it to pre-defined status
    #[returns(QueryResponse)]
    CheckProposalStatus(CheckProposalStatus),

    // Query proposals and check if there're any passed proposals
    #[returns(QueryResponse)]
    CheckPassedProposals { dao_address: String },

    // Query proposals and check if there're any passed proposals with Wasm::Migration message
    #[returns(QueryResponse)]
    CheckWithMigration { dao_address: String },
}
