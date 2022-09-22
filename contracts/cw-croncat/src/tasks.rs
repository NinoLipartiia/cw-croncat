use crate::error::ContractError;
use crate::slots::Interval;
use crate::state::{Config, CwCroncat};
use cosmwasm_std::Storage;
use cosmwasm_std::{
    to_binary, BankMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg};
use cw_croncat_core::error::CoreError;
use cw_croncat_core::msg::{
    GetSlotHashesResponse, GetSlotIdsResponse, TaskRequest, TaskResponse, TaskWithRulesResponse,
};
use cw_croncat_core::traits::{BalancesOperations, Intervals};
use cw_croncat_core::types::{BoundaryValidated, GenericBalance, SlotType, Task};

impl<'a> CwCroncat<'a> {
    /// Returns task data
    /// Used by the frontend for viewing tasks
    pub(crate) fn query_get_tasks(
        &self,
        deps: Deps,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> StdResult<Vec<TaskResponse>> {
        let default_limit = self.config.load(deps.storage)?.limit;
        let size: u64 = self.task_total.load(deps.storage)?.min(default_limit);
        let from_index = from_index.unwrap_or_default();
        let limit = limit.unwrap_or(default_limit).min(size);
        self.tasks
            .range(deps.storage, None, None, Order::Ascending)
            .skip(from_index as usize)
            .take(limit as usize)
            .map(|res| res.map(|(_k, task)| task.into()))
            .collect()
    }

    /// Returns task with rules data
    /// For now it returns only task_hash, interval, boundary and rules,
    /// so that agent doesn't know details of his task
    /// Used by the frontend for viewing tasks
    pub(crate) fn query_get_tasks_with_rules(
        &self,
        deps: Deps,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> StdResult<Vec<TaskWithRulesResponse>> {
        let size: u64 = self.tasks_with_rules_total.load(deps.storage)?.min(1000);
        let from_index = from_index.unwrap_or_default();
        let limit = limit
            .unwrap_or(self.config.load(deps.storage)?.limit)
            .min(size);
        self.tasks_with_rules
            .range(deps.storage, None, None, Order::Ascending)
            .skip(from_index as usize)
            .take(limit as usize)
            .map(|res| res.map(|(_k, task)| task.into()))
            .collect()
    }

    /// Returns task data for a specific owner
    pub(crate) fn query_get_tasks_by_owner(
        &self,
        deps: Deps,
        owner_id: String,
    ) -> StdResult<Vec<TaskResponse>> {
        let owner_id = deps.api.addr_validate(&owner_id)?;
        self.tasks
            .idx
            .owner
            .prefix(owner_id)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|x| x.map(|(_, task)| task.into()))
            .collect::<StdResult<Vec<_>>>()
    }

    /// Returns single task data
    pub(crate) fn query_get_task(
        &self,
        deps: Deps,
        task_hash: String,
    ) -> StdResult<Option<TaskResponse>> {
        let res: Option<Task> = {
            let task = self.tasks.may_load(deps.storage, task_hash.as_bytes())?;
            if let Some(task) = task {
                Some(task)
            } else {
                self.tasks_with_rules
                    .may_load(deps.storage, task_hash.as_bytes())?
            }
        };
        Ok(res.map(Into::into))
    }

    /// Returns a hash computed by the input task data
    pub(crate) fn query_get_task_hash(&self, task: Task) -> StdResult<String> {
        Ok(task.to_hash())
    }

    /// Check if interval params are valid by attempting to parse
    pub(crate) fn query_validate_interval(&self, interval: Interval) -> StdResult<bool> {
        Ok(interval.is_valid())
    }

    /// Gets a set of tasks.
    /// Default: Returns the next executable set of tasks hashes.
    ///
    /// Optional Parameters:
    /// "offset" - An unsigned integer specifying how far in the future to check for tasks that are slotted.
    ///
    /// Result:
    /// (block id, block task hash's, time id, time task hash's)
    pub(crate) fn query_slot_tasks(
        &self,
        deps: Deps,
        slot: Option<u64>,
    ) -> StdResult<GetSlotHashesResponse> {
        let mut block_id: u64 = 0;
        let mut block_hashes: Vec<Vec<u8>> = Vec::new();
        let mut time_id: u64 = 0;
        let mut time_hashes: Vec<Vec<u8>> = Vec::new();

        // Check if slot was supplied, otherwise get the next slots for block and time
        if let Some(id) = slot {
            block_hashes = self
                .block_slots
                .may_load(deps.storage, id)?
                .unwrap_or_default();
            if !block_hashes.is_empty() {
                block_id = id;
            }
            time_hashes = self
                .block_slots
                .may_load(deps.storage, id)?
                .unwrap_or_default();
            if !time_hashes.is_empty() {
                time_id = id;
            }
        } else {
            let time: Vec<(u64, _)> = self
                .time_slots
                .range(deps.storage, None, None, Order::Ascending)
                .take(1)
                .collect::<StdResult<Vec<(u64, _)>>>()?;

            if !time.is_empty() {
                // (time_id, time_hashes) = time[0].clone();
                let slot = time[0].clone();
                time_id = slot.0;
                time_hashes = slot.1;
            }

            let block: Vec<(u64, _)> = self
                .block_slots
                .range(deps.storage, None, None, Order::Ascending)
                .take(1)
                .collect::<StdResult<Vec<(u64, _)>>>()?;

            if !block.is_empty() {
                // (block_id, block_hashes) = block[0].clone();
                let slot = block[0].clone();
                block_id = slot.0;
                block_hashes = slot.1;
            }
        }

        // Generate strings for all hashes
        let block_task_hash: Vec<_> = block_hashes
            .iter()
            .map(|b| String::from_utf8(b.to_vec()).unwrap_or_else(|_| "".to_string()))
            .collect();
        let time_task_hash: Vec<_> = time_hashes
            .iter()
            .map(|t| String::from_utf8(t.to_vec()).unwrap_or_else(|_| "".to_string()))
            .collect();

        Ok(GetSlotHashesResponse {
            block_id,
            block_task_hash,
            time_id,
            time_task_hash,
        })
    }

    /// Gets list of active slot ids, for both time & block slots
    /// (time, block)
    pub(crate) fn query_slot_ids(&self, deps: Deps) -> StdResult<GetSlotIdsResponse> {
        let time_ids: Vec<u64> = self
            .time_slots
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        let block_ids: Vec<u64> = self
            .block_slots
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        Ok(GetSlotIdsResponse {
            time_ids,
            block_ids,
        })
    }

    /// Allows any user or contract to pay for future txns based on a specific schedule
    /// contract, function id & other settings. When the task runs out of balance
    /// the task is no longer executed, any additional funds will be returned to task owner.
    pub fn create_task(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        task: TaskRequest,
    ) -> Result<Response, ContractError> {
        if info.funds.is_empty() {
            return Err(ContractError::CustomError {
                val: "Must attach funds".to_string(),
            });
        }
        let c: Config = self.config.load(deps.storage)?;
        if c.paused {
            return Err(ContractError::CustomError {
                val: "Create task paused".to_string(),
            });
        }

        let owner_id = info.sender;
        let cw20 = if !task.cw20_coins.is_empty() {
            let mut cw20: Vec<Cw20CoinVerified> = Vec::with_capacity(task.cw20_coins.len());
            for coin in task.cw20_coins {
                cw20.push(Cw20CoinVerified {
                    address: deps.api.addr_validate(&coin.address)?,
                    amount: coin.amount,
                })
            }
            // update user balances
            self.balances.update(
                deps.storage,
                &owner_id,
                |balances| -> Result<_, ContractError> {
                    let mut balances = balances.unwrap_or_default();

                    balances.checked_sub_coins(&cw20)?;
                    Ok(balances)
                },
            )?;
            cw20
        } else {
            vec![]
        };
        let boundary = BoundaryValidated::validate_boundary(task.boundary, &task.interval)?;
        let mut item = Task {
            funds_withdrawn_recurring: Uint128::zero(),
            owner_id: owner_id.clone(),
            interval: task.interval,
            boundary,
            stop_on_fail: task.stop_on_fail,
            total_deposit: GenericBalance {
                native: info.funds.clone(),
                cw20,
            },
            amount_for_one_task: Default::default(),
            actions: task.actions,
            rules: task.rules,
        };

        if !item.is_valid_msg_calculate_usage(
            deps.api,
            &env.contract.address,
            &owner_id,
            &c.owner_id,
            c.gas_base_fee,
            c.native_denom.clone(),
        )? {
            return Err(ContractError::CustomError {
                val: "Actions message unsupported or invalid message data".to_string(),
            });
        }

        if !item.interval.is_valid() {
            return Err(ContractError::CustomError {
                val: "Interval invalid".to_string(),
            });
        }

        // Check that balance is sufficient for 1 execution minimum
        let recurring = item.interval != Interval::Once;
        item.verify_enough_balances(recurring)?;

        let hash = item.to_hash();

        // Parse interval into a future timestamp, then convert to a slot
        let (next_id, slot_kind) = item.interval.next(&env, item.boundary);

        // If the next interval comes back 0, then this task should not schedule again
        if next_id == 0 {
            return Err(ContractError::CustomError {
                val: "Task ended".to_string(),
            });
        }

        let mut with_rules = false;
        // Add task to catalog
        if item.with_rules() {
            with_rules = true;
            // Add task with rules
            self.tasks_with_rules
                .update(deps.storage, hash.as_bytes(), |old| match old {
                    Some(_) => Err(ContractError::CustomError {
                        val: "Task already exists".to_string(),
                    }),
                    None => Ok(item.clone()),
                })?;

            // Increment task totals
            let size_res = self.increment_tasks_with_rules(deps.storage);
            if size_res.is_err() {
                return Err(ContractError::CustomError {
                    val: "Problem incrementing task total".to_string(),
                });
            }

            let mut c: Config = c;
            c.available_balance.checked_add_native(&info.funds)?;
            self.config.save(deps.storage, &c)?;

            // Based on slot kind, put into block or cron slots
            match slot_kind {
                SlotType::Block => {
                    self.block_slots_rules
                        .save(deps.storage, hash.as_bytes(), &next_id)?;
                }
                SlotType::Cron => {
                    self.time_slots_rules
                        .save(deps.storage, hash.as_bytes(), &next_id)?;
                }
            }
        } else {
            // Add task without rules
            let hash = item.to_hash_vec();
            self.tasks.update(deps.storage, &hash, |old| match old {
                Some(_) => Err(ContractError::CustomError {
                    val: "Task already exists".to_string(),
                }),
                None => Ok(item),
            })?;

            // Increment task totals
            let size_res = self.increment_tasks(deps.storage);
            if size_res.is_err() {
                return Err(ContractError::CustomError {
                    val: "Problem incrementing task total".to_string(),
                });
            }
            let size = size_res.unwrap();

            // Add the attached balance into available_balance
            let mut c: Config = c;
            c.available_balance.checked_add_native(&info.funds)?;

            // If the creation of this task means we'd like another agent, update config
            // TODO: should we do it for tasks with rules
            let min_tasks_per_agent = c.min_tasks_per_agent;
            let num_active_agents = self.agent_active_queue.load(deps.storage)?.len() as u64;
            let num_agents_to_accept =
                self.agents_to_let_in(&min_tasks_per_agent, &num_active_agents, &size);
            // If we should allow a new agent to take over
            if num_agents_to_accept != 0 {
                // Don't wipe out an older timestamp
                let begin = self.agent_nomination_begin_time.load(deps.storage)?;
                if begin.is_none() {
                    self.agent_nomination_begin_time
                        .save(deps.storage, &Some(env.block.time))?;
                }
            }

            self.config.save(deps.storage, &c)?;

            // Get previous task hashes in slot, add as needed
            let update_vec_data = |d: Option<Vec<Vec<u8>>>| -> StdResult<Vec<Vec<u8>>> {
                match d {
                    // has some data, simply push new hash
                    Some(data) => {
                        let mut s = data;
                        s.push(hash);
                        Ok(s)
                    }
                    // No data, push new vec & hash
                    None => Ok(vec![hash]),
                }
            };

            // Based on slot kind, put into block or cron slots
            match slot_kind {
                SlotType::Block => {
                    self.block_slots
                        .update(deps.storage, next_id, update_vec_data)?;
                }
                SlotType::Cron => {
                    self.time_slots
                        .update(deps.storage, next_id, update_vec_data)?;
                }
            }
        };

        Ok(Response::new()
            .add_attribute("method", "create_task")
            .add_attribute("slot_id", next_id.to_string())
            .add_attribute("slot_kind", format!("{:?}", slot_kind))
            .add_attribute("task_hash", hash)
            .add_attribute("with_rules", with_rules.to_string()))
    }

    /// Deletes a task in its entirety, returning any remaining balance to task owner.
    pub fn remove_task(
        &self,
        storage: &mut dyn Storage,
        task_hash: String,
        info: Option<MessageInfo>,
    ) -> Result<Response, ContractError> {
        let hash_vec = task_hash.clone().into_bytes();
        let some_task = self.tasks.may_load(storage, &hash_vec)?;

        let task = if let Some(task) = some_task {
            if let Some(info) = info {
                if !task.is_owner(info.sender) {
                    return Err(ContractError::Unauthorized {});
                }
            }

            // Remove all the thangs
            self.tasks.remove(storage, &hash_vec)?;

            // find any scheduled things and remove them!
            // check which type of slot it would be in, then iterate to remove
            // NOTE: def could use some spiffy refactor here
            let time_ids: Vec<u64> = self
                .time_slots
                .keys(storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;

            for tid in time_ids {
                let mut time_hashes = self.time_slots.may_load(storage, tid)?.unwrap_or_default();
                if !time_hashes.is_empty() {
                    time_hashes
                        .retain(|h| String::from_utf8(h.to_vec()).unwrap() != task_hash.clone());
                }

                // save the updates, remove if slot no longer has hashes
                if time_hashes.is_empty() {
                    self.time_slots.remove(storage, tid);
                } else {
                    self.time_slots.save(storage, tid, &time_hashes)?;
                }
            }
            let block_ids: Vec<u64> = self
                .block_slots
                .keys(storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;

            for bid in block_ids {
                let mut block_hashes = self.block_slots.may_load(storage, bid)?.unwrap_or_default();
                if !block_hashes.is_empty() {
                    block_hashes
                        .retain(|h| String::from_utf8(h.to_vec()).unwrap() != task_hash.clone());
                }

                // save the updates, remove if slot no longer has hashes
                if block_hashes.is_empty() {
                    self.block_slots.remove(storage, bid);
                } else {
                    self.block_slots.save(storage, bid, &block_hashes)?;
                }
            }
            task
        } else {
            // Find a task with rules
            let task = self
                .tasks_with_rules
                .may_load(storage, &hash_vec)?
                .ok_or(ContractError::NoTaskFound {})?;

            if let Some(info) = info {
                if !task.is_owner(info.sender) {
                    return Err(ContractError::Unauthorized {});
                }
            }

            self.tasks_with_rules.remove(storage, &hash_vec)?;
            self.time_slots_rules.remove(storage, &hash_vec);
            self.block_slots_rules.remove(storage, &hash_vec);

            task
        };

        // return any remaining total_cw20_deposit to the owner
        self.balances.update(
            storage,
            &task.owner_id,
            |balances| -> Result<_, ContractError> {
                let mut balances = balances.unwrap_or_default();
                balances.checked_add_coins(&task.total_deposit.cw20)?;
                Ok(balances)
            },
        )?;
        // remove from the total available_balance
        self.config
            .update(storage, |mut c| -> Result<_, ContractError> {
                c.available_balance
                    .checked_sub_native(&task.total_deposit.native)?;
                Ok(c)
            })?;
        // setup sub-msgs for returning any remaining total_deposit to the owner
        if !task.total_deposit.native.is_empty() {
            Ok(Response::new()
                .add_attribute("method", "remove_task")
                .add_submessage(SubMsg::new(BankMsg::Send {
                    to_address: task.owner_id.into(),
                    amount: task.total_deposit.native,
                })))
        } else {
            Ok(Response::new().add_attribute("method", "remove_task"))
        }
    }

    /// Refill a task with more balance to continue its execution
    /// NOTE: Restricting this to owner only, so owner can make sure the task ends
    pub fn refill_task(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        task_hash: String,
    ) -> Result<Response, ContractError> {
        let hash_vec = task_hash.into_bytes();
        let mut task = self
            .tasks
            .may_load(deps.storage, &hash_vec)?
            .ok_or(ContractError::NoTaskFound {})?;

        if task.owner_id != info.sender {
            return Err(ContractError::RefillNotTaskOwner {});
        }

        // Add the attached balance into available_balance
        let mut c: Config = self.config.load(deps.storage)?;
        c.available_balance.checked_add_native(&info.funds)?;
        task.total_deposit.checked_add_native(&info.funds)?;

        // update the task and the config
        self.config.save(deps.storage, &c)?;
        self.tasks.save(deps.storage, &hash_vec, &task)?;

        // return the task total
        let coins_total: Vec<String> = task
            .total_deposit
            .native
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok(Response::new()
            .add_attribute("method", "refill_task")
            .add_attribute("total_deposit", format!("{coins_total:?}")))
    }

    /// Refill a task with more cw20 balance from user `balance` to continue its execution
    /// NOTE: Restricting this to owner only, so owner can make sure the task ends
    pub fn refill_task_cw20(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        task_hash: String,
        cw20_coins: Vec<Cw20Coin>,
    ) -> Result<Response, ContractError> {
        let task_hash = task_hash.into_bytes();
        let cw20_coins_validated = {
            let mut validated = Vec::with_capacity(cw20_coins.len());
            for coin in cw20_coins {
                validated.push(Cw20CoinVerified {
                    address: deps.api.addr_validate(&coin.address)?,
                    amount: coin.amount,
                })
            }
            validated
        };
        let task = self.tasks.update(deps.storage, &task_hash, |task| {
            let mut task = task.ok_or(ContractError::NoTaskFound {})?;
            if task.owner_id != info.sender {
                return Err(ContractError::RefillNotTaskOwner {});
            }
            // add amount or create with this amount cw20 coins
            task.total_deposit.checked_add_cw20(&cw20_coins_validated)?;
            Ok(task)
        })?;

        // update user balances
        self.balances.update(
            deps.storage,
            &info.sender,
            |balances| -> Result<_, ContractError> {
                let mut balances = balances.unwrap_or_default();
                balances.checked_sub_coins(&cw20_coins_validated)?;
                Ok(balances)
            },
        )?;

        // used `update` here to not clone task_hash

        let total_cw20_string: Vec<String> = task
            .total_deposit
            .cw20
            .iter()
            .map(ToString::to_string)
            .collect();

        Ok(Response::new()
            .add_attribute("method", "refill_task_cw20")
            .add_attribute("total_cw20_deposit", format!("{total_cw20_string:?}")))
    }

    /// Let users withdraw their balances
    pub fn withdraw_wallet_balances(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        cw20_amounts: Vec<Cw20Coin>,
    ) -> Result<Response, ContractError> {
        let wallet = info.sender;
        let withdraws: Vec<Cw20CoinVerified> = {
            let mut withdraws = Vec::with_capacity(cw20_amounts.len());
            for balance in cw20_amounts {
                withdraws.push(Cw20CoinVerified {
                    address: deps.api.addr_validate(&balance.address)?,
                    amount: balance.amount,
                });
            }
            withdraws
        };

        // update user and croncat manager balances
        let new_balances = self.balances.update(
            deps.storage,
            &wallet,
            |balances| -> Result<_, ContractError> {
                let mut balances =
                    balances.ok_or(ContractError::CoreError(CoreError::EmptyBalance {}))?;
                balances.checked_sub_coins(&withdraws)?;
                Ok(balances)
            },
        )?;
        self.config
            .update(deps.storage, |mut c| -> Result<_, ContractError> {
                c.available_balance.checked_sub_cw20(&withdraws)?;
                Ok(c)
            })?;

        let msgs = {
            let mut msgs = Vec::with_capacity(withdraws.len());
            for wd in withdraws {
                msgs.push(WasmMsg::Execute {
                    contract_addr: wd.address.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: wallet.to_string(),
                        amount: wd.amount,
                    })?,
                    funds: vec![],
                });
            }
            msgs
        };

        let new_balances_string: Vec<String> =
            new_balances.iter().map(ToString::to_string).collect();
        Ok(Response::new()
            .add_attribute("method", "withdraw_wallet_balances")
            .add_attribute("total_cw20_deposit", format!("{new_balances_string:?}"))
            .add_messages(msgs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::convert::TryInto;
    // use cosmwasm_std::testing::MockStorage;
    use crate::contract::GAS_BASE_FEE_JUNO;
    use cosmwasm_std::{
        coin, coins, to_binary, Addr, BankMsg, CosmosMsg, Empty, StakingMsg, WasmMsg,
    };
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    use cw_rules_core::types::{HasBalanceGte, Rule};
    // use crate::error::ContractError;
    use crate::helpers::CwTemplateContract;
    use cw_croncat_core::msg::{ExecuteMsg, GetBalancesResponse, InstantiateMsg, QueryMsg};
    use cw_croncat_core::types::{Action, Boundary};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::entry::execute,
            crate::entry::instantiate,
            crate::entry::query,
        );
        Box::new(contract)
    }

    const ADMIN: &str = "cosmos1sjllsnramtg3ewxqwwrwjxfgc4n4ef9u0tvx7u";
    const ANYONE: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const VERY_RICH: &str = "cosmos1c3cy3wzzz3698ypklvh7shksvmefj69xhm89z2";
    const NATIVE_DENOM: &str = "atom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            let accounts: Vec<(u128, String)> = vec![
                (100, ADMIN.to_string()),
                (800_010, ANYONE.to_string()),
                (u128::max_value(), VERY_RICH.to_string()),
            ];
            for (amt, address) in accounts.iter() {
                router
                    .bank
                    .init_balance(
                        storage,
                        &Addr::unchecked(address),
                        vec![coin(amt.clone(), NATIVE_DENOM.to_string())],
                    )
                    .unwrap();
            }
        })
    }

    fn proper_instantiate() -> (App, CwTemplateContract) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());
        let owner_addr = Addr::unchecked(ADMIN);

        let msg = InstantiateMsg {
            denom: "atom".to_string(),
            owner_id: Some(owner_addr.to_string()),
            gas_base_fee: None,
            agent_nomination_duration: Some(360),
            cw_rules_addr: "todo".to_string(),
        };
        let cw_template_contract_addr = app
            .instantiate_contract(cw_template_id, owner_addr, &msg, &[], "Manager", None)
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    #[test]
    fn query_task_hash_success() {
        let (app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let to_address = String::from("you");
        let amount = coins(1015, "earth");
        let bank = BankMsg::Send { to_address, amount };
        let msg: CosmosMsg = bank.clone().into();

        let task = Task {
            funds_withdrawn_recurring: Uint128::zero(),
            owner_id: Addr::unchecked("nobody".to_string()),
            interval: Interval::Immediate,
            boundary: BoundaryValidated {
                start: None,
                end: None,
            },
            stop_on_fail: false,
            total_deposit: GenericBalance {
                native: coins(37, "atom"),
                cw20: Default::default(),
            },
            amount_for_one_task: Default::default(),
            actions: vec![Action {
                msg,
                gas_limit: Some(150_000),
            }],
            rules: None,
        };

        // HASH CHECK!
        let task_hash: String = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTaskHash {
                    task: Box::new(task),
                },
            )
            .unwrap();
        assert_eq!(
            "69217dd2b6334abe2544a12fcb89588f9cc5c62a298b8720706d9befa3d736d3",
            task_hash
        );
    }

    #[test]
    fn query_validate_interval_success() {
        let (app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let intervals: Vec<Interval> = vec![
            Interval::Once,
            Interval::Immediate,
            Interval::Block(12345),
            Interval::Cron("0 0 * * * *".to_string()),
        ];
        for i in intervals.iter() {
            let valid: bool = app
                .wrap()
                .query_wasm_smart(
                    &contract_addr.clone(),
                    &QueryMsg::ValidateInterval {
                        interval: i.to_owned(),
                    },
                )
                .unwrap();
            assert!(valid);
        }
    }

    #[test]
    fn query_get_tasks() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };

        // create a task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(300010, "atom"),
        )
        .unwrap();

        // check storage has the task
        let all_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(all_tasks.len(), 1);

        let owner_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasksByOwner {
                    owner_id: ANYONE.to_string(),
                },
            )
            .unwrap();
        assert_eq!(owner_tasks.len(), 1);
    }

    #[test]
    fn query_get_tasks_pagination() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let tasks_amnt: u64 = 10;
        let from_index = 3;
        let limit = 2;
        let new_msg = |amount| ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg: StakingMsg::Delegate {
                        validator: validator.clone(),
                        amount: coin(amount, "atom"),
                    }
                    .into(),
                    gas_limit: Some(150_000),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };

        // create a tasks
        for amount in 1..tasks_amnt as u128 + 1 {
            app.execute_contract(
                Addr::unchecked(VERY_RICH),
                contract_addr.clone(),
                &new_msg(amount),
                &coins(300000 + 2 * amount, "atom"),
            )
            .unwrap();
        }
        let mut all_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(all_tasks.len(), tasks_amnt as usize);

        // check we get right amount of tasks
        let part_of_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: Some(from_index),
                    limit: None,
                },
            )
            .unwrap();
        let expected_amnt: usize = (tasks_amnt - from_index).try_into().unwrap();
        assert_eq!(part_of_tasks.len(), expected_amnt);

        println!(
            "half_tasks: {:?}\n hash_vec:{:?}",
            part_of_tasks
                .iter()
                .map(|t| t.task_hash.clone())
                .collect::<Vec<String>>(),
            all_tasks
                .iter()
                .map(|t| t.task_hash.clone())
                .collect::<Vec<String>>(),
        );

        // Check it's in right order
        for i in 0..expected_amnt {
            assert_eq!(
                all_tasks[from_index as usize + i].task_hash,
                part_of_tasks[i].task_hash
            );
        }

        // and with limit
        let part_of_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: Some(from_index),
                    limit: Some(limit),
                },
            )
            .unwrap();
        let expected_amnt: usize = (limit).try_into().unwrap();
        assert_eq!(part_of_tasks.len(), expected_amnt);

        // Edge cases

        // Index out of bounds, so we return nothing
        let from_index = tasks_amnt;
        let out_of_bounds: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: Some(from_index),
                    limit: None,
                },
            )
            .unwrap();
        assert!(out_of_bounds.is_empty());

        // Returns as many elements as possible without a panic
        let from_index = tasks_amnt - 2;
        let two_last_elements: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: Some(from_index),
                    limit: Some(tasks_amnt),
                },
            )
            .unwrap();
        assert_eq!(two_last_elements.len(), 2);

        // Removed task shouldn't reorder things
        let removed_index = from_index as usize;
        app.execute_contract(
            Addr::unchecked(VERY_RICH),
            contract_addr.clone(),
            &ExecuteMsg::RemoveTask {
                task_hash: all_tasks
                    .remove(removed_index) // We removed hash from original vector to match
                    .task_hash,
            },
            &vec![],
        )
        .unwrap();
        let new_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(new_tasks, all_tasks);
    }

    #[test]
    fn check_task_create_fail_cases() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg: msg.clone(),
                    gas_limit: Some(150_000),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };
        // let task_id_str = "95c916a53fa9d26deef094f7e1ee31c00a2d47b8bf474b2e06d39aebfb1fecc7".to_string();
        // let task_id = task_id_str.clone().into_bytes();

        // Must attach funds
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &vec![],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Must attach funds".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Create task paused
        let change_settings_msg = ExecuteMsg::UpdateSettings {
            paused: Some(true),
            owner_id: None,
            // treasury_id: None,
            agent_fee: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
            min_tasks_per_agent: None,
        };
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &change_settings_msg,
            &vec![],
        )
        .unwrap();
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(300010, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Create task paused".to_string()
            },
            res_err.downcast().unwrap()
        );
        // Set it back
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSettings {
                paused: Some(false),
                owner_id: None,
                // treasury_id: None,
                agent_fee: None,
                agents_eject_threshold: None,
                gas_price: None,
                proxy_callback_gas: None,
                slot_granularity: None,
                min_tasks_per_agent: None,
            },
            &vec![],
        )
        .unwrap();

        // Creator invalid
        let action_self = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.clone().into_string(),
            funds: vec![],
            msg: to_binary(&change_settings_msg.clone())?,
        });
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::CreateTask {
                    task: TaskRequest {
                        interval: Interval::Once,
                        boundary: None,
                        stop_on_fail: false,
                        actions: vec![Action {
                            msg: action_self.clone(),
                            gas_limit: Some(150_000),
                        }],
                        rules: None,
                        cw20_coins: vec![],
                    },
                },
                &coins(13, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Actions message unsupported or invalid message data".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Interval invalid
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::CreateTask {
                    task: TaskRequest {
                        interval: Interval::Cron("faux_paw".to_string()),
                        boundary: None,
                        stop_on_fail: false,
                        actions: vec![Action {
                            msg: msg.clone(),
                            gas_limit: Some(150_000),
                        }],
                        rules: None,
                        cw20_coins: vec![],
                    },
                },
                &coins(13, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Interval invalid".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Task already exists
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(300010, "atom"),
        )
        .unwrap();
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(300010, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Task already exists".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Task ended
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::CreateTask {
                    task: TaskRequest {
                        interval: Interval::Block(12346),
                        boundary: Some(Boundary::Height {
                            start: None,
                            end: Some(1u64.into()),
                        }),
                        stop_on_fail: false,
                        actions: vec![Action {
                            msg,
                            gas_limit: Some(150_000),
                        }],
                        rules: None,
                        cw20_coins: vec![],
                    },
                },
                &coins(300010, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Task ended".to_string()
            },
            res_err.downcast().unwrap()
        );

        // TODO: (needs impl!) Not enough task balance to execute job

        Ok(())
    }

    #[test]
    fn check_task_create_success() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };
        let task_id_str =
            "95c916a53fa9d26deef094f7e1ee31c00a2d47b8bf474b2e06d39aebfb1fecc7".to_string();

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(300010, "atom"),
            )
            .unwrap();
        // Assert task hash is returned as part of event attributes
        let mut has_created_hash: bool = false;
        for e in res.events {
            for a in e.attributes {
                if a.key == "task_hash" && a.value == task_id_str.clone() {
                    has_created_hash = true;
                }
            }
        }
        assert!(has_created_hash);

        // check storage has the task
        let new_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(new_task.is_some());
        if let Some(t) = new_task {
            assert_eq!(Addr::unchecked(ANYONE), t.owner_id);
            assert_eq!(Interval::Immediate, t.interval);
            assert_eq!(None, t.boundary);
            assert_eq!(false, t.stop_on_fail);
            assert_eq!(coins(300010, "atom"), t.total_deposit);
            assert_eq!(task_id_str.clone(), t.task_hash);
        }

        // get slot ids
        let slot_ids: GetSlotIdsResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetSlotIds {})
            .unwrap();
        let s_1: Vec<u64> = Vec::new();
        assert_eq!(s_1, slot_ids.time_ids);
        assert_eq!(vec![12346], slot_ids.block_ids);

        // get slot hashs
        let slot_info: GetSlotHashesResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetSlotHashes { slot: None },
            )
            .unwrap();
        let s_3: Vec<String> = Vec::new();
        assert_eq!(12346, slot_info.block_id);
        assert_eq!(vec![task_id_str.clone()], slot_info.block_task_hash);
        assert_eq!(0, slot_info.time_id);
        assert_eq!(s_3, slot_info.time_task_hash);

        Ok(())
    }

    #[test]
    fn check_task_with_rules_create_success() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: Some(vec![Rule::HasBalanceGte(HasBalanceGte {
                    address: "foo".to_string(),
                    required_balance: coins(5, "bar").into(),
                })]),
                cw20_coins: vec![],
            },
        };

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(300010, "atom"),
            )
            .unwrap();

        let tasks_with_rules: Vec<TaskWithRulesResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasksWithRules {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();

        let tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(tasks_with_rules.len(), 1);
        assert_eq!(tasks.len(), 0);

        let mut has_created_hash: bool = false;
        for e in res.events {
            for a in e.attributes {
                if a.key == "with_rules" && a.value == "true" {
                    has_created_hash = true;
                }
            }
        }
        assert!(has_created_hash);
        Ok(())
    }

    #[test]
    fn check_task_with_rules_and_without_create_success() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let with_rules_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg: msg.clone(),
                    gas_limit: Some(150_000),
                }],
                rules: Some(vec![Rule::HasBalanceGte(HasBalanceGte {
                    address: "foo".to_string(),
                    required_balance: coins(5, "bar").into(),
                })]),
                cw20_coins: vec![],
            },
        };

        let without_rules_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &with_rules_msg,
                &coins(300010, "atom"),
            )
            .unwrap();

        let res2 = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &without_rules_msg,
                &coins(300010, "atom"),
            )
            .unwrap();

        let tasks_with_rules: Vec<TaskWithRulesResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasksWithRules {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();

        let tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(tasks_with_rules.len(), 1);
        assert_eq!(tasks.len(), 1);

        let mut has_created_hash: bool = false;
        for e in res.events {
            for a in e.attributes {
                if a.key == "with_rules" && a.value == "true" {
                    has_created_hash = true;
                }
            }
        }

        res2.events.into_iter().any(|ev| {
            ev.attributes
                .into_iter()
                .any(|attr| attr.key == "with_rules" && attr.value == "false")
        });
        assert!(has_created_hash);
        Ok(())
    }

    #[test]
    fn check_remove_create() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };
        let task_id_str =
            "95c916a53fa9d26deef094f7e1ee31c00a2d47b8bf474b2e06d39aebfb1fecc7".to_string();

        // create a task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(300010, "atom"),
        )
        .unwrap();

        // check storage DOES have the task
        let new_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(new_task.is_some());

        // Confirm slot exists, proving task was scheduled
        let slot_ids: GetSlotIdsResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetSlotIds {})
            .unwrap();
        let s_1: Vec<u64> = Vec::new();
        assert_eq!(s_1, slot_ids.time_ids);
        assert_eq!(vec![12346], slot_ids.block_ids);

        // Another person can't remove the task
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::RemoveTask {
                task_hash: task_id_str.clone(),
            },
            &vec![],
        )
        .unwrap_err();

        // Remove the Task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &ExecuteMsg::RemoveTask {
                task_hash: task_id_str.clone(),
            },
            &vec![],
        )
        .unwrap();

        // check storage DOESNT have the task
        let rem_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(rem_task.is_none());

        // Check the contract total balance has decreased from the removed task
        let balances: GetBalancesResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetBalances {})
            .unwrap();
        assert_eq!(balances.available_balance.native, vec![]);

        // Check the slots correctly removed the task
        let slot_ids: GetSlotIdsResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetSlotIds {})
            .unwrap();
        let s: Vec<u64> = Vec::new();
        assert_eq!(s.clone(), slot_ids.time_ids);
        assert_eq!(s, slot_ids.block_ids);

        Ok(())
    }

    #[test]
    fn check_refill_create() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };
        let task_id_str =
            "95c916a53fa9d26deef094f7e1ee31c00a2d47b8bf474b2e06d39aebfb1fecc7".to_string();

        // create a task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(300010, "atom"),
        )
        .unwrap();
        // refill task
        let res = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::RefillTaskBalance {
                    task_hash: task_id_str.clone(),
                },
                &coins(3, "atom"),
            )
            .unwrap();
        // Assert returned event attributes include total
        let mut matches_new_totals: bool = false;
        for e in res.events {
            for a in e.attributes {
                if a.key == "total_deposit" && a.value == r#"["300013atom"]"#.to_string() {
                    matches_new_totals = true;
                }
            }
        }
        assert!(matches_new_totals);

        // check the task totals
        let new_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(new_task.is_some());

        if let Some(t) = new_task {
            assert_eq!(Addr::unchecked(ANYONE), t.owner_id);
            assert_eq!(coins(300013, "atom"), t.total_deposit);
        }

        // Check the balance has increased to include the new refilled total
        let balances: GetBalancesResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetBalances {})
            .unwrap();
        assert_eq!(coins(300013, "atom"), balances.available_balance.native);

        Ok(())
    }

    #[test]
    fn check_gas_minimum() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();
        let gas_limit = 150_000;

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(gas_limit),
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };
        // create 1 token off task
        let amount_for_one_task = gas_limit + 3;
        let res: ContractError = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(u128::from(amount_for_one_task * 2 - 1), "atom"),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            res,
            ContractError::CoreError(CoreError::NotEnoughNative {
                denom: "atom".to_string(),
                lack: Uint128::from(1u128)
            })
        );

        // create a task
        let res = app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(u128::from(amount_for_one_task * 2), "atom"),
        );
        assert!(res.is_ok());
    }

    #[test]
    fn check_gas_default() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();
        let gas_limit = GAS_BASE_FEE_JUNO;

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: None,
                }],
                rules: None,
                cw20_coins: vec![],
            },
        };
        // create 1 token off task
        // for one task need gas + staking amount
        let amount_for_one_task = gas_limit + 3;
        let res: ContractError = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(u128::from(amount_for_one_task * 2 - 1), "atom"),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            res,
            ContractError::CoreError(CoreError::NotEnoughNative {
                denom: "atom".to_string(),
                lack: Uint128::from(1u128)
            })
        );

        // create a task
        let res = app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(u128::from(amount_for_one_task * 2), "atom"),
        );
        assert!(res.is_ok());
    }
}
