pub mod agent;
pub mod balancer;
pub mod contract;
mod error;
pub mod helpers;
pub mod manager;
pub mod owner;
pub mod receiver;
pub mod slots;
pub mod state;
pub mod tasks;
pub mod traits;
pub use crate::error::ContractError;
pub use crate::state::CwCroncat;
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult};
pub use cw_croncat_core::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    // This makes a conscious choice on the various generics used by the contract
    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let s = CwCroncat::default();
        s.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        let mut s = CwCroncat::default();
        s.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let mut s = CwCroncat::default();
        s.query(deps, env, msg)
    }

    #[entry_point]
    pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        let s = CwCroncat::default();
        s.reply(deps, env, msg)
    }
}
