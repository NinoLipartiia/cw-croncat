use cosmwasm_std::{
    coin, Addr, Api, BankMsg, Coin, CosmosMsg, Empty, Env, GovMsg, IbcMsg, OverflowError,
    OverflowOperation::Sub, StakingMsg, StdError, SubMsgResult, Timestamp, Uint128, Uint64,
    WasmMsg,
};
use cron_schedule::Schedule;
use cw20::{Cw20CoinVerified, Cw20ExecuteMsg};
use cw_rules_core::types::Rule;
use hex::encode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

use crate::{
    error::CoreError,
    traits::{BalancesOperations, FindAndMutate, Intervals, ResultFailed},
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum AgentStatus {
    // Default for any new agent, if tasks ratio allows
    Active,

    // Default for any new agent, until more tasks come online
    Pending,

    // More tasks are available, agent must checkin to become active
    Nominated,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Agent {
    // Where rewards get transferred
    pub payable_account_id: Addr,

    // accrued reward balance
    pub balance: GenericBalance,

    // stats
    pub total_tasks_executed: u64,

    // Holds slot number of a missed slot.
    // If other agents see an agent miss a slot, they store the missed slot number.
    // If agent does a task later, this number is reset to zero.
    // Example data: 1633890060000000000 or 0
    pub last_missed_slot: u64,

    // Timestamp of when agent first registered
    // Useful for rewarding agents for their patience while they are pending and operating service
    // Agent will be responsible to constantly monitor when it is their turn to join in active agent set (done as part of agent code loops)
    // Example data: 1633890060000000000 or 0
    pub register_start: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AgentResponse {
    // This field doesn't exist in the Agent struct and is the only one that differs
    pub status: AgentStatus,
    pub payable_account_id: Addr,
    pub balance: GenericBalance,
    pub total_tasks_executed: u64,
    pub last_missed_slot: u64,
    pub register_start: Timestamp,
}

/// Defines the spacing of execution
/// NOTE:S
/// - Block Height Based: Once, Immediate, Block
/// - Timestamp Based: Cron
/// - No Epoch support directly, advised to use block heights instead
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum Interval {
    /// For when this is a non-recurring future scheduled TXN
    Once,

    /// The ugly batch schedule type, in case you need to exceed single TXN gas limits, within fewest block(s)
    Immediate,

    /// Allows timing based on block intervals rather than timestamps
    Block(u64),

    /// Crontab Spec String
    Cron(String),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum Boundary {
    Height {
        start: Option<Uint64>,
        end: Option<Uint64>,
    },
    Time {
        start: Option<Timestamp>,
        end: Option<Timestamp>,
    },
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BoundaryValidated {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl BoundaryValidated {
    pub fn validate_boundary(
        boundary: Option<Boundary>,
        interval: &Interval,
    ) -> Result<Self, CoreError> {
        if let Some(boundary) = boundary {
            match (interval, boundary) {
                (Interval::Cron(_), Boundary::Time { start, end }) => Ok(Self {
                    start: start.map(|start| start.nanos()),
                    end: end.map(|end| end.nanos()),
                }),
                (
                    Interval::Once | Interval::Immediate | Interval::Block(_),
                    Boundary::Height { start, end },
                ) => Ok(Self {
                    start: start.map(Into::into),
                    end: end.map(Into::into),
                }),
                _ => Err(CoreError::InvalidBoundary {}),
            }
        } else {
            Ok(Self {
                start: None,
                end: None,
            })
        }
    }
}

#[derive(
    Debug, PartialEq, Eq, std::hash::Hash, Deserialize, Serialize, Clone, Copy, JsonSchema,
)]
pub enum SlotType {
    Block,
    Cron,
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
// pub struct Rule {
//     /// TBD: Interchain query support (See ibc::IbcMsg)
//     // pub chain_id: Option<String>,

//     /// Account to direct all view calls against
//     pub contract_addr: String,

//     // NOTE: Only allow static pre-defined query msg
//     pub msg: Binary,
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Action<T = Empty> {
    // NOTE: Only allow static pre-defined query msg
    /// Supported CosmosMsgs only!
    pub msg: CosmosMsg<T>,

    /// The gas needed to safely process the execute msg
    pub gas_limit: Option<u64>,
}

impl Action {
    // Checking how much native coins sent in this action
    pub fn bank_sent(&self) -> Option<&[Coin]> {
        if let CosmosMsg::Bank(BankMsg::Send { amount, .. }) = &self.msg {
            Some(amount)
        } else {
            None
        }
    }

    // Checking how much cw20 coins sent in this action
    pub fn cw20_sent(&self, api: &dyn Api) -> Option<Cw20CoinVerified> {
        if let CosmosMsg::Wasm(WasmMsg::Execute {
            msg, contract_addr, ..
        }) = &self.msg
        {
            if let Ok(cw20_msg) = cosmwasm_std::from_binary(msg) {
                return match cw20_msg {
                    Cw20ExecuteMsg::Send { amount, .. } => Some(Cw20CoinVerified {
                        // unwraping safe here because we checked it at `is_valid_msg_calculate_usage`
                        address: api.addr_validate(contract_addr).unwrap(),
                        amount,
                    }),
                    Cw20ExecuteMsg::Transfer { amount, .. } => Some(Cw20CoinVerified {
                        address: api.addr_validate(contract_addr).unwrap(),
                        amount,
                    }),
                    _ => None,
                };
            }
        }
        None
    }
}

/// The response required by all rule queries. Bool is needed for croncat, T allows flexible rule engine
pub type RuleResponse<T> = (bool, T);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Task {
    /// Entity responsible for this task, can change task details
    pub owner_id: Addr,

    /// Scheduling definitions
    pub interval: Interval,
    pub boundary: BoundaryValidated,
    pub funds_withdrawn_recurring: Uint128,

    /// Defines if this task can continue until balance runs out
    pub stop_on_fail: bool,

    /// NOTE: Only tally native balance here, manager can maintain token/balances outside of tasks
    pub total_deposit: GenericBalance,

    pub amount_for_one_task: GenericBalance,

    /// The cosmos message to call, if time or rules are met
    pub actions: Vec<Action>,
    /// A prioritized list of messages that can be chained decision matrix
    /// required to complete before task action
    /// Rules MUST return the ResolverResponse type
    pub rules: Option<Vec<Rule>>,
    // TODO: funds! should we support funds being attached?
}

impl Task {
    /// Get the hash of a task based on parameters
    pub fn to_hash(&self) -> String {
        let message = format!(
            "{:?}{:?}{:?}{:?}{:?}",
            self.owner_id, self.interval, self.boundary, self.actions, self.rules
        );

        let hash = Sha256::digest(message.as_bytes());
        encode(hash)
    }
    /// Get the hash of a task based on parameters
    pub fn to_hash_vec(&self) -> Vec<u8> {
        self.to_hash().into_bytes()
    }

    pub fn verify_enough_balances(&self, recurring: bool) -> Result<(), CoreError> {
        let multiplier = Uint128::from(if recurring { 2u128 } else { 1u128 });

        let task_native_balance_uses = &self.amount_for_one_task.native;
        for coin in task_native_balance_uses {
            if let Some(balance) = self
                .total_deposit
                .native
                .iter()
                .find(|balance| balance.denom == coin.denom)
            {
                if balance.amount < coin.amount * multiplier {
                    return Err(CoreError::NotEnoughNative {
                        denom: coin.denom.clone(),
                        lack: coin.amount * multiplier - balance.amount,
                    });
                }
            } else {
                return Err(CoreError::NotEnoughNative {
                    denom: coin.denom.clone(),
                    lack: coin.amount,
                });
            }
        }
        let task_cw20_balance_uses = &self.amount_for_one_task.cw20;
        for coin in task_cw20_balance_uses {
            if let Some(balance) = self
                .total_deposit
                .cw20
                .iter()
                .find(|balance| balance.address == coin.address)
            {
                if balance.amount < coin.amount * multiplier {
                    return Err(CoreError::NotEnoughCw20 {
                        addr: coin.address.to_string(),
                        lack: coin.amount * multiplier - balance.amount,
                    });
                }
            } else {
                return Err(CoreError::NotEnoughCw20 {
                    addr: coin.address.to_string(),
                    lack: coin.amount,
                });
            }
        }
        Ok(())
    }

    pub fn is_recurring(&self) -> bool {
        matches!(&self.interval, Interval::Cron(_) | Interval::Block(_))
    }

    pub fn contains_send_msg(&self) -> bool {
        let result: bool = self.actions.iter().any(|a| -> bool {
            matches!(
                &a.msg,
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: _,
                    amount: _,
                })
            )
        });
        result
    }

    /// Validate the task actions only use the supported messages
    /// We're iterating over all actions
    /// so it's a great place for calculaing balance usages
    pub fn is_valid_msg_calculate_usage(
        &mut self,
        api: &dyn Api,
        self_addr: &Addr,
        sender: &Addr,
        owner_id: &Addr,
        default_gas: u64,
        native_denom: String,
    ) -> Result<bool, CoreError> {
        // TODO: Chagne to default FALSE, once all messages are covered in tests
        let mut valid = true;
        let mut gas_amount: u64 = 0;
        let amount_for_one_task = &mut self.amount_for_one_task;

        for action in self.actions.iter() {
            // checked for cases, where task creator intentionaly tries to overflow
            gas_amount = gas_amount
                .checked_add(action.gas_limit.unwrap_or(default_gas))
                .ok_or(CoreError::InvalidWasmMsg {})?;
            match &action.msg {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    funds: _,
                    msg,
                }) => {
                    // TODO: Is there any way sender can be "self" creating a malicious task?
                    // cannot be THIS contract id, unless predecessor is owner of THIS contract
                    if contract_addr == self_addr && sender != owner_id {
                        valid = false;
                    }
                    if let Ok(cw20_msg) = cosmwasm_std::from_binary(msg) {
                        match cw20_msg {
                            Cw20ExecuteMsg::Send { amount, .. } if !amount.is_zero() => {
                                amount_for_one_task
                                    .cw20
                                    .find_checked_add(&Cw20CoinVerified {
                                        address: api.addr_validate(contract_addr)?,
                                        amount,
                                    })?
                            }
                            Cw20ExecuteMsg::Transfer { amount, .. } if !amount.is_zero() => {
                                amount_for_one_task
                                    .cw20
                                    .find_checked_add(&Cw20CoinVerified {
                                        address: api.addr_validate(contract_addr)?,
                                        amount,
                                    })?
                            }
                            _ => valid = false,
                        }
                    }
                }
                CosmosMsg::Staking(StakingMsg::Delegate {
                    validator: _,
                    amount,
                }) => {
                    // Must attach enough balance for staking
                    if amount.amount.is_zero() {
                        valid = false;
                    }
                    amount_for_one_task.native.find_checked_add(amount)?;
                }
                // TODO: Allow send, as long as coverage of assets is correctly handled
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: _,
                    amount,
                }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    // Do something silly to keep it simple. Ensure they only sent one kind of native token and it's testnet Juno
                    // Remember total_deposit is set in tasks.rs when a task is created, and assigned to info.funds
                    // which is however much was passed in, like 1000000ujunox below:
                    // junod tx wasm execute … … --amount 1000000ujunox
                    if amount.iter().any(|coin| coin.amount.is_zero()) {
                        valid = false;
                    }
                    amount_for_one_task.checked_add_native(amount)?;
                }
                CosmosMsg::Bank(BankMsg::Burn { .. }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    valid = false;
                }
                CosmosMsg::Gov(GovMsg::Vote { .. }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    valid = false;
                }
                // TODO: Setup better support for IBC
                CosmosMsg::Ibc(IbcMsg::Transfer { .. }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    valid = false;
                }
                // TODO: Check authZ messages
                _ => (),
            }
        }
        amount_for_one_task
            .native
            .find_checked_add(&coin(gas_amount as u128, native_denom))?;
        Ok(valid)
    }

    /// Get task gas total
    /// helper for getting total configured gas for this tasks actions
    pub fn to_gas_total(&self) -> u64 {
        let mut gas: u64 = 0;

        // tally all the gases
        for action in self.actions.iter() {
            gas = gas.saturating_add(action.gas_limit.unwrap_or(0));
        }

        gas
    }

    /// Get whether the task is with rules
    pub fn with_rules(&self) -> bool {
        self.rules.is_some() && !self.rules.as_ref().unwrap().is_empty()
    }

    /// Check if given Addr is the owner
    pub fn is_owner(&self, addr: Addr) -> bool {
        self.owner_id == addr
    }
}

impl FindAndMutate<'_, Coin> for Vec<Coin> {
    fn find_checked_add(&mut self, add: &Coin) -> Result<(), CoreError> {
        let token = self.iter_mut().find(|exist| exist.denom == add.denom);
        match token {
            Some(exist) => {
                exist.amount = exist
                    .amount
                    .checked_add(add.amount)
                    .map_err(StdError::overflow)?
            }
            None => self.push(add.clone()),
        }
        Ok(())
    }

    fn find_checked_sub(&mut self, sub: &Coin) -> Result<(), CoreError> {
        let coin = self.iter().position(|exist| exist.denom == sub.denom);
        match coin {
            Some(exist) => {
                match self[exist].amount.cmp(&sub.amount) {
                    std::cmp::Ordering::Less => {
                        return Err(CoreError::Std(StdError::overflow(OverflowError::new(
                            Sub,
                            self[exist].amount,
                            sub.amount,
                        ))))
                    }
                    std::cmp::Ordering::Equal => {
                        self.swap_remove(exist);
                    }
                    std::cmp::Ordering::Greater => self[exist].amount -= sub.amount,
                };
                Ok(())
            }
            None => Err(CoreError::EmptyBalance {}),
        }
    }
}

impl FindAndMutate<'_, Cw20CoinVerified> for Vec<Cw20CoinVerified> {
    fn find_checked_add(&mut self, add: &Cw20CoinVerified) -> Result<(), CoreError> {
        let token = self.iter_mut().find(|exist| exist.address == add.address);
        match token {
            Some(exist) => {
                exist.amount = exist
                    .amount
                    .checked_add(add.amount)
                    .map_err(StdError::overflow)?
            }
            None => self.push(add.clone()),
        }
        Ok(())
    }

    fn find_checked_sub(&mut self, sub: &Cw20CoinVerified) -> Result<(), CoreError> {
        let coin_p = self.iter().position(|exist| exist.address == sub.address);
        match coin_p {
            Some(exist) => {
                match self[exist].amount.cmp(&sub.amount) {
                    std::cmp::Ordering::Less => {
                        return Err(CoreError::Std(StdError::overflow(OverflowError::new(
                            Sub,
                            self[exist].amount,
                            sub.amount,
                        ))))
                    }
                    std::cmp::Ordering::Equal => {
                        self.swap_remove(exist);
                    }
                    std::cmp::Ordering::Greater => self[exist].amount -= sub.amount,
                };
                Ok(())
            }
            None => Err(CoreError::EmptyBalance {}),
        }
    }
}

impl<'a, T, Rhs> BalancesOperations<'a, T, Rhs> for Vec<T>
where
    Rhs: IntoIterator<Item = &'a T>,
    Self: FindAndMutate<'a, T>,
    T: 'a,
{
    fn checked_add_coins(&mut self, add: Rhs) -> Result<(), CoreError> {
        for add_token in add {
            self.find_checked_add(add_token)?;
        }
        Ok(())
    }

    fn checked_sub_coins(&mut self, sub: Rhs) -> Result<(), CoreError> {
        for sub_token in sub {
            self.find_checked_sub(sub_token)?;
        }
        Ok(())
    }
}

impl GenericBalance {
    pub fn checked_add_native(&mut self, add: &[Coin]) -> Result<(), CoreError> {
        self.native.checked_add_coins(add)
    }

    pub fn checked_add_cw20(&mut self, add: &[Cw20CoinVerified]) -> Result<(), CoreError> {
        self.cw20.checked_add_coins(add)
    }

    pub fn checked_sub_native(&mut self, sub: &[Coin]) -> Result<(), CoreError> {
        self.native.checked_sub_coins(sub)
    }

    pub fn checked_sub_cw20(&mut self, sub: &[Cw20CoinVerified]) -> Result<(), CoreError> {
        self.cw20.checked_sub_coins(sub)
    }

    pub fn checked_sub_generic(&mut self, sub: &GenericBalance) -> Result<(), CoreError> {
        self.checked_sub_native(&sub.native)?;
        self.checked_sub_cw20(&sub.cw20)
    }
}

impl ResultFailed for SubMsgResult {
    fn failed(&self) -> bool {
        match self {
            SubMsgResult::Ok(response) => response.events.iter().any(|event| {
                event.attributes.iter().any(|attribute| {
                    event.ty == "reply"
                        && attribute.key == "mode"
                        && attribute.value == "handle_failure"
                })
            }),
            SubMsgResult::Err(_) => true,
        }
    }
}

fn get_next_block_limited(env: &Env, boundary: BoundaryValidated) -> (u64, SlotType) {
    let current_block_height = env.block.height;

    let next_block_height = match boundary.start {
        // shorthand - remove 1 since it adds 1 later
        Some(id) if current_block_height < id => id - 1,
        _ => current_block_height,
    };

    match boundary.end {
        // stop if passed end height
        Some(id) if current_block_height > id => (0, SlotType::Block),

        // we ONLY want to catch if we're passed the end block height
        Some(id) if next_block_height > id => (id, SlotType::Block),

        // immediate needs to return this block + 1
        _ => (next_block_height + 1, SlotType::Block),
    }
}

// So either:
// - Boundary specifies a start/end that block offsets can be computed from
// - Block offset will truncate to specific modulo offsets
fn get_next_block_by_offset(env: &Env, boundary: BoundaryValidated, block: u64) -> (u64, SlotType) {
    let current_block_height = env.block.height;
    let modulo_block = current_block_height.saturating_sub(current_block_height % block) + block;

    let next_block_height = match boundary.start {
        Some(id) if current_block_height < id => {
            let rem = id % block;
            if rem > 0 {
                id.saturating_sub(rem) + block
            } else {
                id
            }
        }
        _ => modulo_block,
    };

    match boundary.end {
        // stop if passed end height
        Some(id) if current_block_height > id => (0, SlotType::Block),

        // we ONLY want to catch if we're passed the end block height
        Some(id) => {
            let end_height = if let Some(rem) = id.checked_rem(block) {
                id.saturating_sub(rem)
            } else {
                id
            };
            (end_height, SlotType::Block)
        }

        None => (next_block_height, SlotType::Block),
    }
}

impl Intervals for Interval {
    fn next(&self, env: &Env, boundary: BoundaryValidated) -> (u64, SlotType) {
        match self {
            // return the first block within a specific range that can be triggered 1 time.
            Interval::Once => get_next_block_limited(env, boundary),
            // return the first block within a specific range that can be triggered immediately, potentially multiple times.
            Interval::Immediate => get_next_block_limited(env, boundary),
            // return the first block within a specific range that can be triggered 1 or more times based on timestamps.
            // Uses crontab spec
            Interval::Cron(crontab) => {
                let current_block_ts: u64 = env.block.time.nanos();
                // TODO: get current timestamp within boundary
                let current_ts = match boundary.start {
                    Some(ts) if current_block_ts < ts => ts,
                    _ => current_block_ts,
                };
                let schedule = Schedule::from_str(crontab.as_str()).unwrap();
                let next_ts = schedule.next_after(&current_ts).unwrap();
                (next_ts, SlotType::Cron)
            }
            // return the block within a specific range that can be triggered 1 or more times based on block heights.
            // Uses block offset (Example: Block(100) will trigger every 100 blocks)
            // So either:
            // - Boundary specifies a start/end that block offsets can be computed from
            // - Block offset will truncate to specific modulo offsets
            Interval::Block(block) => get_next_block_by_offset(env, boundary, *block),
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Interval::Once => true,
            Interval::Immediate => true,
            Interval::Block(_) => true,
            Interval::Cron(crontab) => {
                let s = Schedule::from_str(crontab);
                s.is_ok()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        coins, testing::mock_dependencies, Binary, IbcTimeout, Uint128, VoteOption,
    };
    use cw_rules_core::types::HasBalanceGte;
    use hex::ToHex;

    #[test]
    fn is_valid_msg_once_block_based() {
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Once,
            boundary: BoundaryValidated {
                start: Some(4),
                end: Some(8),
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "alice".to_string(),
                    msg: Binary::from(vec![]),
                    funds: vec![Coin::new(10, "coin")],
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(task
            .is_valid_msg_calculate_usage(
                &mock_dependencies().api,
                &Addr::unchecked("alice2"),
                &Addr::unchecked("bob"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_once_time_based() {
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Once,
            boundary: BoundaryValidated {
                start: Some(1_000_000_000),
                end: Some(2_000_000_000),
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "alice".to_string(),
                    msg: Binary::from(vec![]),
                    funds: vec![Coin::new(10, "coin")],
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(task
            .is_valid_msg_calculate_usage(
                &mock_dependencies().api,
                &Addr::unchecked("alice2"),
                &Addr::unchecked("bob"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_recurring() {
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Block(10),
            boundary: BoundaryValidated {
                start: None,
                end: None,
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "alice".to_string(),
                    msg: Binary::from(vec![]),
                    funds: vec![Coin::new(10, "coin")],
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(task
            .is_valid_msg_calculate_usage(
                &mock_dependencies().api,
                &Addr::unchecked("alice2"),
                &Addr::unchecked("bob"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_wrong_account() {
        // Cannot create a task to execute on the cron manager when not the owner
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("alice"),
            interval: Interval::Block(5),
            boundary: BoundaryValidated {
                start: Some(4),
                end: None,
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "alice".to_string(),
                    msg: Binary::from(vec![]),
                    funds: vec![Coin::new(10, "coin")],
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(!task
            .is_valid_msg_calculate_usage(
                &mock_dependencies().api,
                &Addr::unchecked("alice"),
                &Addr::unchecked("sender"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_vote() {
        // A task with CosmosMsg::Gov Vote should return false
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Block(5),
            boundary: BoundaryValidated {
                start: Some(4),
                end: None,
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Gov(GovMsg::Vote {
                    proposal_id: 0,
                    vote: VoteOption::Yes,
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(!task
            .is_valid_msg_calculate_usage(
                &mock_dependencies().api,
                &Addr::unchecked("alice"),
                &Addr::unchecked("sender"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_transfer() {
        // A task with CosmosMsg::Ibc Transfer should return false
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Block(5),
            boundary: BoundaryValidated {
                start: Some(4),
                end: None,
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Ibc(IbcMsg::Transfer {
                    channel_id: "id".to_string(),
                    to_address: "address".to_string(),
                    amount: Coin::new(10, "coin"),
                    timeout: IbcTimeout::with_timestamp(Timestamp::from_nanos(1_000_000_000)),
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(!task
            .is_valid_msg_calculate_usage(
                &mock_dependencies().api,
                &Addr::unchecked("alice"),
                &Addr::unchecked("sender"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_burn() {
        // A task with CosmosMsg::Bank Burn should return false
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Block(5),
            boundary: BoundaryValidated {
                start: Some(4),
                end: None,
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Bank(BankMsg::Burn {
                    amount: vec![Coin::new(10, "coin")],
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(!task
            .is_valid_msg_calculate_usage(
                mock_dependencies().as_ref().api,
                &Addr::unchecked("alice"),
                &Addr::unchecked("sender"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_send_doesnt_fail() {
        // A task with CosmosMsg::Bank Send should return true
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Block(5),
            boundary: BoundaryValidated {
                start: Some(4),
                end: None,
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "address".to_string(),
                    amount: vec![Coin::new(10, "coin")],
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(task
            .is_valid_msg_calculate_usage(
                mock_dependencies().as_ref().api,
                &Addr::unchecked("alice"),
                &Addr::unchecked("sender"),
                &Addr::unchecked("bob"),
                100,
                "coin".to_string()
            )
            .unwrap());
    }

    #[test]
    fn is_valid_msg_send_should_success() {
        // A task with CosmosMsg::Bank Send should return false
        let mut task = Task {
            funds_withdrawn_recurring: Uint128::zero(),

            owner_id: Addr::unchecked("bob"),
            interval: Interval::Block(1),
            boundary: BoundaryValidated {
                start: Some(4),
                end: None,
            },
            stop_on_fail: false,
            total_deposit: GenericBalance {
                native: coins(10, "atom"),
                cw20: Default::default(),
            },
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "address".to_string(),
                    amount: vec![Coin::new(10, "atom")],
                }),
                gas_limit: Some(5),
            }],
            rules: None,
        };
        assert!(task
            .is_valid_msg_calculate_usage(
                mock_dependencies().as_ref().api,
                &Addr::unchecked("alice"),
                &Addr::unchecked("sender"),
                &Addr::unchecked("bob"),
                100,
                "atom".to_string()
            )
            .unwrap());
    }

    #[test]
    fn test_add_tokens() {
        let mut coins: GenericBalance = GenericBalance::default();

        // Adding zero doesn't change the state
        let add_zero: Vec<Coin> = vec![];
        coins.checked_add_native(&add_zero).unwrap();
        assert!(coins.native.is_empty());
        assert!(coins.cw20.is_empty());

        // Check that we can add native coin for the first time
        let add_native = vec![Coin::new(10, "native")];
        coins.checked_add_native(&add_native).unwrap();
        assert_eq!(coins.native.len(), 1);
        assert_eq!(coins.native, add_native);
        assert!(coins.cw20.is_empty());

        // Check that we can add the same native coin again
        let add_native = vec![Coin::new(20, "native")];
        coins.checked_add_native(&add_native).unwrap();
        assert_eq!(coins.native.len(), 1);
        assert_eq!(coins.native, vec![Coin::new(30, "native")]);
        assert!(coins.cw20.is_empty());

        // Check that we can add a coin for the first time
        let cw20 = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (1000_u128).into(),
        };
        let add_cw20: Vec<Cw20CoinVerified> = vec![cw20.clone()];
        coins.checked_add_cw20(&add_cw20).unwrap();
        assert_eq!(coins.native.len(), 1);
        assert_eq!(coins.native, vec![Coin::new(30, "native")]);
        assert_eq!(coins.cw20.len(), 1);
        assert_eq!(coins.cw20[0], cw20);

        // Check that we can add the same coin again
        let cw20 = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (2000_u128).into(),
        };
        let add_cw20: Vec<Cw20CoinVerified> = vec![cw20];
        coins.checked_add_cw20(&add_cw20).unwrap();
        assert_eq!(coins.native.len(), 1);
        assert_eq!(coins.native, vec![Coin::new(30, "native")]);
        assert_eq!(coins.cw20.len(), 1);
        let cw20_result = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (3000_u128).into(),
        };
        assert_eq!(coins.cw20[0], cw20_result);
    }

    #[test]
    fn test_add_tokens_overflow_native() {
        let mut coins: GenericBalance = GenericBalance::default();
        // Adding one coin
        let add_native = vec![Coin::new(1, "native")];
        coins.checked_add_native(&add_native).unwrap();

        // Adding u128::MAX amount should fail
        let add_max = vec![Coin::new(u128::MAX, "native")];
        let err = coins.checked_add_native(&add_max).unwrap_err();
        assert!(matches!(err, CoreError::Std(StdError::Overflow { .. })))
    }

    #[test]
    fn test_add_tokens_overflow_cw20() {
        let mut coins: GenericBalance = GenericBalance::default();
        // Adding one coin
        let cw20 = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (1_u128).into(),
        };
        let add_cw20 = vec![cw20];
        coins.checked_add_cw20(&add_cw20).unwrap();

        // Adding u128::MAX amount should fail
        let cw20_max = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: u128::MAX.into(),
        };
        let add_max: Vec<Cw20CoinVerified> = vec![cw20_max];
        let err = coins.checked_add_cw20(&add_max).unwrap_err();
        assert!(matches!(err, CoreError::Std(StdError::Overflow { .. })))
    }

    #[test]
    fn test_minus_tokens() {
        let mut coins: GenericBalance = GenericBalance::default();

        // Adding some native and cw20 tokens
        let add_native = vec![Coin::new(100, "native")];
        coins.checked_add_native(&add_native).unwrap();

        let cw20 = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (100_u128).into(),
        };
        let add_cw20 = vec![cw20];
        coins.checked_add_cw20(&add_cw20).unwrap();

        // Check subtraction of native token
        let minus_native = vec![Coin::new(10, "native")];
        coins.checked_sub_native(&minus_native).unwrap();
        assert_eq!(coins.native, vec![Coin::new(90, "native")]);

        // Check subtraction of cw20
        let cw20 = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (20_u128).into(),
        };
        let minus_cw20 = vec![cw20];
        coins.checked_sub_cw20(&minus_cw20).unwrap();
        let cw20_result = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (80_u128).into(),
        };
        assert_eq!(coins.cw20[0], cw20_result);
    }

    #[test]
    fn test_minus_tokens_overflow_native() {
        let mut coins: GenericBalance = GenericBalance::default();

        // Adding some native tokens
        let add_native = vec![Coin::new(100, "native")];
        coins.checked_add_native(&add_native).unwrap();

        // Substracting more than added should fail
        let minus_native = vec![Coin::new(101, "native")];
        let err = coins.checked_sub_native(&minus_native).unwrap_err();

        assert!(matches!(err, CoreError::Std(StdError::Overflow { .. })))
    }

    #[test]
    fn test_minus_tokens_overflow_cw20() {
        let mut coins: GenericBalance = GenericBalance::default();

        // Adding some cw20 tokens
        let cw20 = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (100_u128).into(),
        };
        let add_cw20 = vec![cw20];
        coins.checked_add_cw20(&add_cw20).unwrap();

        // Substracting more than added should fail
        let cw20 = Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: (101_u128).into(),
        };
        let minus_cw20 = vec![cw20];
        let err = coins.checked_sub_cw20(&minus_cw20).unwrap_err();

        assert!(matches!(err, CoreError::Std(StdError::Overflow { .. })))
    }

    #[test]
    fn hashing() {
        let task = Task {
            funds_withdrawn_recurring: Uint128::zero(),
            owner_id: Addr::unchecked("bob"),
            interval: Interval::Block(5),
            boundary: BoundaryValidated {
                start: Some(4),
                end: None,
            },
            stop_on_fail: false,
            total_deposit: Default::default(),
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg: CosmosMsg::Wasm(WasmMsg::ClearAdmin {
                    contract_addr: "alice".to_string(),
                }),
                gas_limit: Some(5),
            }],
            rules: Some(vec![Rule::HasBalanceGte(HasBalanceGte {
                address: "foo".to_string(),
                required_balance: coins(5, "atom").into(),
            })]),
        };

        let message = format!(
            "{:?}{:?}{:?}{:?}{:?}",
            task.owner_id, task.interval, task.boundary, task.actions, task.rules
        );

        let hash = Sha256::digest(message.as_bytes());

        let encoded: String = hash.encode_hex();
        let bytes = encoded.as_bytes();

        // Tests
        assert_eq!(encoded, task.to_hash());
        assert_eq!(bytes, task.to_hash_vec());
    }
}
