use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// We can import dao but for simplicity we show what we support
pub mod dao {
    pub use cw_rules_core::types::Status;

    use super::*;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryDao {
        Proposal {
            proposal_id: u64,
        },
        ListProposals {
            start_after: Option<u64>,
            limit: Option<u64>,
        },
        ProposalCount {},
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
    pub struct ProposalResponse {
        /// The ID of the proposal being returned.
        pub id: u64,
        pub proposal: AnyChoiceProposal,
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
    pub struct ProposalListResponse {
        pub proposals: Vec<ProposalResponse>,
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
    pub struct AnyChoiceProposal {
        pub status: Status,
        //Ignore rest
    }
}
