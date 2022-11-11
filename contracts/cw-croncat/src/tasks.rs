use crate::error::ContractError;
use crate::slots::Interval;
use crate::state::{Config, CwCroncat};
use cosmwasm_std::{coin, Storage};
use cosmwasm_std::{
    to_binary, BankMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, SubMsg,
    WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg};
use cw_croncat_core::error::CoreError;
use cw_croncat_core::msg::{
    GetSlotHashesResponse, GetSlotIdsResponse, TaskRequest, TaskResponse, TaskWithRulesResponse,
};
use cw_croncat_core::traits::{BalancesOperations, FindAndMutate, Intervals};
use cw_croncat_core::types::{
    calculate_required_amount, BoundaryValidated, GenericBalance, SlotType, Task,
};

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
        let cfg: Config = self.config.load(deps.storage)?;
        if cfg.paused {
            return Err(ContractError::CustomError {
                val: "Create task paused".to_string(),
            });
        }

        let owner_id = &info.sender;
        let cw20 = if !task.cw20_coins.is_empty() {
            let mut cw20: Vec<Cw20CoinVerified> = Vec::with_capacity(task.cw20_coins.len());
            for coin in &task.cw20_coins {
                cw20.push(Cw20CoinVerified {
                    address: deps.api.addr_validate(&coin.address)?,
                    amount: coin.amount,
                })
            }
            // update user balances
            self.balances.update(
                deps.storage,
                owner_id,
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

        if !task.interval.is_valid() {
            return Err(ContractError::CustomError {
                val: "Interval invalid".to_string(),
            });
        }

        let (mut amount_for_one_task, gas_amount) = task.is_valid_msg_calculate_usage(
            deps.api,
            &env.contract.address,
            owner_id,
            &cfg.owner_id,
            cfg.gas_base_fee,
            cfg.gas_action_fee,
        )?;
        let gas_price = calculate_required_amount(gas_amount, cfg.agent_fee)?;
        let price = cfg.gas_fraction.calculate(gas_price, 1)?;
        amount_for_one_task
            .native
            .find_checked_add(&coin(price, &cfg.native_denom))?;

        let item = Task {
            funds_withdrawn_recurring: vec![],
            owner_id: owner_id.clone(),
            interval: task.interval,
            boundary,
            stop_on_fail: task.stop_on_fail,
            total_deposit: GenericBalance {
                native: info.funds.clone(),
                cw20,
            },
            amount_for_one_task,
            actions: task.actions,
            rules: task.rules,
        };

        // Check that balance is sufficient for 1 execution minimum
        let recurring = item.interval != Interval::Once;
        item.verify_enough_balances(recurring)?;
        // Add the attached balance into available_balance
        let cfg = self
            .config
            .update(deps.storage, |mut c| -> Result<_, ContractError> {
                c.available_balance.checked_add_native(&info.funds)?;
                Ok(c)
            })?;

        let hash = item.to_hash();

        // Parse interval into a future timestamp, then convert to a slot
        let (next_id, slot_kind) =
            item.interval
                .next(&env, item.boundary, cfg.slot_granularity_time);

        // If the next interval comes back 0, then this task should not schedule again
        if next_id == 0 {
            return Err(ContractError::CustomError {
                val: "Task ended".to_string(),
            });
        }

        let with_rules = item.with_rules();
        // Add task to catalog
        if with_rules {
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

            // Based on slot kind, put into block or cron slots
            match slot_kind {
                SlotType::Block => {
                    self.block_map_rules
                        .save(deps.storage, hash.as_bytes(), &next_id)?;
                }
                SlotType::Cron => {
                    self.time_map_rules
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

            // If the creation of this task means we'd like another agent, update config
            // TODO: should we do it for tasks with rules
            let min_tasks_per_agent = cfg.min_tasks_per_agent;
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
        task_hash: &str,
        info: Option<MessageInfo>,
    ) -> Result<Response, ContractError> {
        let hash_vec = task_hash.as_bytes().to_vec();
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
            // check which type of slot  it would be in, then iterate to remove
            // NOTE: def could use some spiffy refactor here
            let time_ids: Vec<u64> = self
                .time_slots
                .keys(storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;

            for tid in time_ids {
                let mut time_hashes = self.time_slots.may_load(storage, tid)?.unwrap_or_default();
                if !time_hashes.is_empty() {
                    time_hashes.retain(|h| h != &hash_vec);
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
                    block_hashes.retain(|h| h != &hash_vec);
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
            self.pop_task_with_rule(storage, hash_vec, info)?
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

    fn pop_task_with_rule(
        &self,
        storage: &mut dyn Storage,
        hash_vec: Vec<u8>,
        info: Option<MessageInfo>,
    ) -> Result<Task, ContractError> {
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
        match task.interval {
            Interval::Cron(_) => self.time_map_rules.remove(storage, &hash_vec),
            _ => self.block_map_rules.remove(storage, &hash_vec),
        }
        Ok(task)
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
