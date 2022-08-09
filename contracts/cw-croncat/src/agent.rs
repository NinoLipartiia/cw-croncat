use crate::balancer::Balancer;
use crate::error::ContractError;
use crate::helpers::{send_tokens, GenericBalance};
use crate::state::{Config, CwCroncat};
use cosmwasm_std::{
    has_coins, Addr, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage,
    SubMsg, Uint64,
};
use std::ops::Div;

use crate::ContractError::AgentNotRegistered;
use cw_croncat_core::msg::{AgentTaskResponse, GetAgentIdsResponse};
use cw_croncat_core::types::{Agent, AgentResponse, AgentStatus};

impl<'a> CwCroncat<'a> {
    /// Get a single agent details
    /// Check's status as well, in case this agent needs to be considered for election
    pub(crate) fn query_get_agent(
        &self,
        deps: Deps,
        env: Env,
        account_id: Addr,
    ) -> StdResult<Option<AgentResponse>> {
        let agent = self.agents.may_load(deps.storage, account_id.clone())?;
        if agent.is_none() {
            return Ok(None);
        }
        let active: Vec<Addr> = self.agent_active_queue.load(deps.storage)?;
        let a = agent.unwrap();
        let mut agent_response = AgentResponse {
            status: AgentStatus::Pending, // Simple default
            payable_account_id: a.payable_account_id,
            balance: a.balance,
            total_tasks_executed: a.total_tasks_executed,
            last_missed_slot: a.last_missed_slot,
            register_start: a.register_start,
        };

        if active.contains(&account_id) {
            agent_response.status = AgentStatus::Active;
            return Ok(Some(agent_response));
        }

        let agent_status = self.get_agent_status(deps.storage, env, account_id);

        // Return wrapped error if there was a problem
        if agent_status.is_err() {
            return Err(StdError::GenericErr {
                msg: agent_status.err().unwrap().to_string(),
            });
        }

        agent_response.status = agent_status.expect("Should have valid agent status");
        Ok(Some(agent_response))
    }

    /// Get a list of agent addresses
    pub(crate) fn query_get_agent_ids(&self, deps: Deps) -> StdResult<GetAgentIdsResponse> {
        let active: Vec<Addr> = self.agent_active_queue.load(deps.storage)?;
        let pending: Vec<Addr> = self.agent_pending_queue.load(deps.storage)?;

        Ok(GetAgentIdsResponse { active, pending })
    }

    // TODO: Change this to solid round-table implementation. Setup this simple version for PoC
    /// Get how many tasks an agent can execute
    /// TODO: Remove this function, replaced by balancer
    pub(crate) fn query_get_agent_tasks(
        &mut self,
        deps: Deps,
        env: Env,
        account_id: Addr,
    ) -> StdResult<Option<AgentTaskResponse>> {
        let active = self.agent_active_queue.load(deps.storage)?;
        if !active.contains(&account_id) {
            // TODO: unsure if we can return AgentNotRegistered
            return Err(StdError::GenericErr {
                msg: AgentNotRegistered {}.to_string(),
            });
        }

        // Get all tasks (the final None means no limit when we take)
        let slot_items = self.get_current_slot_items(&env.block, deps.storage, None);

        if slot_items == (None, None) {
            return Ok(None);
        }
        let mut num_block_tasks = Uint64::from(0u64);
        let mut num_cron_tasks = Uint64::from(0u64);
        let num_block_tasks_extra = Uint64::from(0u64);
        let num_cron_tasks_extra = Uint64::from(0u64);
        // This below line is commented out and will be used with
        // the rotating index (see Config's agent_active_indices)
        // let agent_active_queue_indices: Vec<usize> = (0..active.len()).collect();
        if let Some(current_block_task_total) = slot_items.0 {
            // Integer division to determine how much each gets
            let task_total_each_agent = current_block_task_total / active.len() as u64;

            // Divvy up the modulo leftovers using the active index
            // TODO: we must give the leftover tasks to some agents.
            // Still need to implement the "round table" idea, where we use Config's agent_active_indices
            // let agent_active_index = agent_active_queue
            //     .iter()
            //     .position(|x| x == &account_id)
            //     .expect("Agent not active");
            // let leftover_tasks = total_tasks % agent_active_queue.len() as u64;

            num_block_tasks = task_total_each_agent.into();
        }
        // Do time slots
        if let Some(current_cron_task_total) = slot_items.1 {
            // Integer division to determine how much each gets
            let task_total_each_agent = current_cron_task_total / active.len() as u64;

            // Divvy up the modulo leftovers using the active index
            // TODO: we must give the leftover tasks to some agents.
            // Still need to implement the "round table" idea, where we use Config's agent_active_indices
            // let agent_active_index = agent_active_queue
            //     .iter()
            //     .position(|x| x == &account_id)
            //     .expect("Agent not active");
            // let leftover_tasks = total_tasks % agent_active_queue.len() as u64;

            num_cron_tasks = task_total_each_agent.into();
        }

        Ok(Some(AgentTaskResponse {
            num_block_tasks,
            num_block_tasks_extra,
            num_cron_tasks_extra,
            num_cron_tasks,
        }))
    }

    /// Add any account as an agent that will be able to execute tasks.
    /// Registering allows for rewards accruing with micro-payments which will accumulate to more long-term.
    ///
    /// Optional Parameters:
    /// "payable_account_id" - Allows a different account id to be specified, so a user can receive funds at a different account than the agent account.
    pub fn register_agent(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        payable_account_id: Option<Addr>,
    ) -> Result<Response, ContractError> {
        if !info.funds.is_empty() {
            return Err(ContractError::CustomError {
                val: "Do not attach funds".to_string(),
            });
        }
        let c: Config = self.config.load(deps.storage)?;
        if c.paused {
            return Err(ContractError::ContractPaused {
                val: "Register agent paused".to_string(),
            });
        }

        let account = info.sender;

        // REF: https://github.com/CosmWasm/cw-tokens/tree/main/contracts/cw20-escrow
        // Check if native token balance is sufficient for a few txns, in this case 4 txns
        // TODO: Adjust gas & costs based on real usage cost
        let agent_wallet_balances = deps.querier.query_all_balances(account.clone())?;
        let unit_cost = c.gas_price * 4;
        if !has_coins(
            &agent_wallet_balances,
            &Coin::new(u128::from(unit_cost), c.native_denom),
        ) || agent_wallet_balances.is_empty()
        {
            return Err(ContractError::CustomError {
                val: "Insufficient funds".to_string(),
            });
        }

        let payable_id = payable_account_id.unwrap_or_else(|| account.clone());

        let mut active_agents: Vec<Addr> = self.agent_active_queue.load(deps.storage)?;
        let total_agents = active_agents.len();
        let agent_status = if total_agents == 0 {
            active_agents.push(account.clone());
            self.agent_active_queue.save(deps.storage, &active_agents)?;
            AgentStatus::Active
        } else {
            let mut pending_agents = self.agent_pending_queue.load(deps.storage)?;
            pending_agents.push(account.clone());
            self.agent_pending_queue
                .save(deps.storage, &pending_agents)?;
            AgentStatus::Pending
        };

        self.agents.update(
            deps.storage,
            account,
            |a: Option<Agent>| -> Result<_, ContractError> {
                match a {
                    // make sure that account isn't already added
                    Some(_) => Err(ContractError::CustomError {
                        val: "Agent already exists".to_string(),
                    }),
                    None => {
                        Ok(Agent {
                            payable_account_id: payable_id,
                            balance: GenericBalance::default(),
                            total_tasks_executed: 0,
                            last_missed_slot: 0,
                            // REF: https://github.com/CosmWasm/cosmwasm/blob/main/packages/std/src/types.rs#L57
                            register_start: env.block.time,
                        })
                    }
                }
            },
        )?;

        Ok(Response::new()
            .add_attribute("method", "register_agent")
            .add_attribute("agent_status", format!("{:?}", agent_status))
            .add_attribute("register_start", env.block.time.nanos().to_string()))
    }

    /// Update agent details, specifically the payable account id for an agent.
    pub fn update_agent(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        _env: Env,
        payable_account_id: Addr,
    ) -> Result<Response, ContractError> {
        let c: Config = self.config.load(deps.storage)?;
        if c.paused {
            return Err(ContractError::ContractPaused {
                val: "Register agent paused".to_string(),
            });
        }

        self.agents.update(
            deps.storage,
            info.sender,
            |a: Option<Agent>| -> Result<_, ContractError> {
                match a {
                    Some(agent) => {
                        let mut ag = agent;
                        ag.payable_account_id = payable_account_id;
                        Ok(ag)
                    }
                    None => Err(ContractError::AgentNotRegistered {}),
                }
            },
        )?;

        Ok(Response::new().add_attribute("method", "update_agent"))
    }

    /// Allows an agent to withdraw all rewards, paid to the specified payable account id.
    pub(crate) fn withdraw_balances(
        &self,
        storage: &mut dyn Storage,
        info: MessageInfo,
    ) -> Result<Vec<SubMsg>, ContractError> {
        let a = self.agents.may_load(storage, info.sender)?;
        if a.is_none() {
            return Err(ContractError::AgentNotRegistered {});
        }
        let agent = a.unwrap();

        // This will send all token balances to Agent
        let (messages, balances) = send_tokens(&agent.payable_account_id, &agent.balance)?;
        let mut config = self.config.load(storage)?;
        config
            .available_balance
            .checked_sub_native(&balances.native)?;
        // TODO: Finish:
        // config
        //     .available_balance
        //     .minus_tokens(Balance::from(balances.cw20));
        self.config.save(storage, &config)?;

        Ok(messages)
    }

    /// Allows an agent to withdraw all rewards, paid to the specified payable account id.
    pub fn withdraw_agent_balance(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        _env: Env,
    ) -> Result<Response, ContractError> {
        let messages = self.withdraw_balances(deps.storage, info.clone())?;

        Ok(Response::new()
            .add_attribute("method", "withdraw_agent_balance")
            .add_attribute("account_id", info.sender)
            .add_submessages(messages))
    }

    /// Allows an agent to accept a nomination within a certain amount of time to become an active agent.
    pub fn accept_nomination_agent(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
    ) -> Result<Response, ContractError> {
        // Compare current time and Config's agent_nomination_begin_time to see if agent can join
        let c: Config = self.config.load(deps.storage)?;

        let time_difference =
            if let Some(nomination_start) = self.agent_nomination_begin_time.load(deps.storage)? {
                env.block.time.seconds() - nomination_start.seconds()
            } else {
                // No agents can join yet
                return Err(ContractError::CustomError {
                    val: "Not accepting new agents".to_string(),
                });
            };
        // Agent must be in the pending queue
        let pending_queue = self.agent_pending_queue.load(deps.storage)?;
        // Get the position in the pending queue
        if let Some(agent_position) = pending_queue
            .iter()
            .position(|address| address == &info.sender)
        {
            // It works out such that the time difference between when this is called,
            // and the agent nomination begin time can be divided by the nomination
            // duration and we get an integer. We use that integer to determine if an
            // agent is allowed to get let in. If their position in the pending queue is
            // less than or equal to that integer, they get let in.
            let max_index = time_difference.div(c.agent_nomination_duration as u64);
            if agent_position as u64 <= max_index {
                // Make this agent active
                // Update state removing from pending queue
                let mut pending_agents: Vec<Addr> = self.agent_pending_queue.load(deps.storage)?;
                // Remove this agent and all ahead of them in the queue (they missed out)
                for idx_to_remove in (0..=agent_position).rev() {
                    pending_agents.remove(idx_to_remove);
                }
                self.agent_pending_queue
                    .save(deps.storage, &pending_agents)?;

                // and adding to active queue
                let mut active_agents: Vec<Addr> = self.agent_active_queue.load(deps.storage)?;
                active_agents.push(info.sender.clone());
                self.agent_active_queue.save(deps.storage, &active_agents)?;

                // and update the config, setting the nomination begin time to None,
                // which indicates no one will be nominated until more tasks arrive
                self.agent_nomination_begin_time.save(deps.storage, &None)?;
                self.config.save(deps.storage, &c)?;
            } else {
                return Err(ContractError::CustomError {
                    val: "Must wait longer before accepting nomination".to_string(),
                });
            }
        } else {
            // Sender's address does not exist in the agent pending queue
            return Err(ContractError::AgentNotRegistered {});
        }
        // Find difference
        Ok(Response::new().add_attribute("method", "accept_nomination_agent"))
    }

    /// Removes the agent from the active set of agents.
    /// Withdraws all reward balances to the agent payable account id.
    pub fn unregister_agent(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        _env: Env,
    ) -> Result<Response, ContractError> {
        // Get withdraw messages, if any
        // NOTE: Since this also checks if agent exists, safe to not have redundant logic
        let messages = self.withdraw_balances(deps.storage, info.clone())?;
        let agent_id = info.sender;
        self.agents.remove(deps.storage, agent_id.clone());

        // Remove from the list of active agents if the agent in this list
        let mut active_agents: Vec<Addr> = self
            .agent_active_queue
            .may_load(deps.storage)?
            .unwrap_or_default();
        if let Some(index) = active_agents.iter().position(|addr| *addr == agent_id) {
            //Notify the balancer agent has been removed, to rebalance itself
            self.balancer.on_agent_unregister(
                deps.storage,
                &self.config,
                &self.agent_active_queue,
                agent_id.clone(),
            );
            active_agents.remove(index);

            self.agent_active_queue.save(deps.storage, &active_agents)?;
        } else {
            // Agent can't be both in active and pending vector
            // Remove from the pending queue
            let mut pending_agents: Vec<Addr> = self
                .agent_pending_queue
                .may_load(deps.storage)?
                .unwrap_or_default();
            if let Some(index) = pending_agents.iter().position(|addr| *addr == agent_id) {
                pending_agents.remove(index);
                self.agent_pending_queue
                    .save(deps.storage, &pending_agents)?;
            }
        }

        let responses = Response::new()
            .add_attribute("method", "unregister_agent")
            .add_attribute("account_id", agent_id);

        if messages.is_empty() {
            Ok(responses)
        } else {
            Ok(responses.add_submessages(messages))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ContractError;
    use crate::helpers::CwTemplateContract;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, from_slice, Addr, BlockInfo, CosmosMsg, Empty, StakingMsg};
    use cw_croncat_core::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TaskRequest, TaskResponse};
    use cw_croncat_core::types::{Action, Interval};
    use cw_multi_test::{App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::entry::execute,
            crate::entry::instantiate,
            crate::entry::query,
        );
        Box::new(contract)
    }

    const AGENT0: &str = "cosmos1a7uhnpqthunr2rzj0ww0hwurpn42wyun6c5puz";
    const AGENT1: &str = "cosmos17muvdgkep4ndptnyg38eufxsssq8jr3wnkysy8";
    const AGENT2: &str = "cosmos1qxywje86amll9ptzxmla5ah52uvsd9f7drs2dl";
    const AGENT3: &str = "cosmos1c3cy3wzzz3698ypklvh7shksvmefj69xhm89z2";
    const AGENT4: &str = "cosmos1ykfcyj8fl6xzs88tsls05x93gmq68a7km05m4j";
    const AGENT_BENEFICIARY: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const ADMIN: &str = "cosmos1sjllsnramtg3ewxqwwrwjxfgc4n4ef9u0tvx7u";
    const PARTICIPANT0: &str = "cosmos1055rfv3fv0zxsp8h3x88mctnm7x9mlgmf4m4d6";
    const PARTICIPANT1: &str = "cosmos1c3cy3wzzz3698ypklvh7shksvmefj69xhm89z2";
    const PARTICIPANT2: &str = "cosmos1far5cqkvny7k9wq53aw0k42v3f76rcylzzv05n";
    const PARTICIPANT3: &str = "cosmos1xj3xagnprtqpfnvyp7k393kmes73rpuxqgamd8";
    const PARTICIPANT4: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const PARTICIPANT5: &str = "cosmos1k5k7y4hgy5lkq0kj3k3e9k38lquh0m66kxsu5c";
    const PARTICIPANT6: &str = "cosmos14a8clxc49z9e3mjzhamhkprt2hgf0y53zczzj0";
    const NATIVE_DENOM: &str = "atom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            let accounts: Vec<(u128, String)> = vec![
                (2_000_000, ADMIN.to_string()),
                (1, AGENT0.to_string()),
                (2_000_000, AGENT1.to_string()),
                (2_000_000, AGENT2.to_string()),
                (2_000_000, AGENT3.to_string()),
                (2_000_000, AGENT4.to_string()),
                (500_0000, PARTICIPANT0.to_string()),
                (500_0000, PARTICIPANT1.to_string()),
                (500_0000, PARTICIPANT2.to_string()),
                (500_0000, PARTICIPANT3.to_string()),
                (500_0000, PARTICIPANT4.to_string()),
                (500_0000, PARTICIPANT5.to_string()),
                (500_0000, PARTICIPANT6.to_string()),
                (2_000_000, AGENT_BENEFICIARY.to_string()),
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
            owner_id: Some(owner_addr.clone()),
            gas_base_fee: None,
            agent_nomination_duration: Some(360),
        };
        let cw_template_contract_addr = app
            .instantiate_contract(cw_template_id, owner_addr, &msg, &[], "Manager", None)
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    fn get_task_total(app: &App, contract_addr: &Addr) -> usize {
        let res: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                contract_addr,
                &QueryMsg::GetTasks {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();
        res.len()
    }

    fn add_task_exec(app: &mut App, contract_addr: &Addr, sender: &str) -> AppResponse {
        let validator = String::from("you");
        let amount = coin(3, NATIVE_DENOM);
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();
        let send_funds = coins(500_000, NATIVE_DENOM);
        app.execute_contract(
            Addr::unchecked(sender),
            contract_addr.clone(),
            &ExecuteMsg::CreateTask {
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
            },
            send_funds.as_ref(),
        )
        .expect("Error adding task")
    }

    fn add_block_task_exec(
        app: &mut App,
        contract_addr: &Addr,
        sender: &str,
        block_num: u64,
    ) -> AppResponse {
        let validator = String::from("you");
        let amount = coin(3, NATIVE_DENOM);
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();
        let send_funds = coins(500_000, NATIVE_DENOM);
        app.execute_contract(
            Addr::unchecked(sender),
            contract_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: TaskRequest {
                    interval: Interval::Block(block_num),
                    boundary: None,
                    stop_on_fail: false,
                    actions: vec![Action {
                        msg,
                        gas_limit: Some(150_000),
                    }],
                    rules: None,
                    cw20_coins: vec![],
                },
            },
            send_funds.as_ref(),
        )
        .expect("Error adding task")
    }

    fn add_cron_task_exec(
        app: &mut App,
        contract_addr: &Addr,
        sender: &str,
        num_minutes: u64,
    ) -> AppResponse {
        let validator = String::from("you");
        let amount = coin(3, NATIVE_DENOM);
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();
        let send_funds = coins(500_000, NATIVE_DENOM);
        app.execute_contract(
            Addr::unchecked(sender),
            contract_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: TaskRequest {
                    interval: Interval::Cron(format!("* {} * * * *", num_minutes)),
                    boundary: None,
                    stop_on_fail: false,
                    actions: vec![Action {
                        msg,
                        gas_limit: Some(150_000),
                    }],
                    rules: None,
                    cw20_coins: vec![],
                },
            },
            send_funds.as_ref(),
        )
        .expect("Error adding task")
    }

    fn contract_create_task(
        contract: &CwCroncat,
        deps: DepsMut,
        info: &MessageInfo,
    ) -> Result<Response, ContractError> {
        // try adding task without app
        let validator = String::from("you");
        let amount = coin(3, NATIVE_DENOM);
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();
        // let send_funds = coins(1, NATIVE_DENOM);

        contract.create_task(
            deps,
            info.clone(),
            mock_env(),
            TaskRequest {
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
        )
    }

    fn contract_register_agent(
        sender: &str,
        contract: &mut CwCroncat,
        deps: DepsMut,
    ) -> Result<Response, ContractError> {
        contract.execute(
            deps,
            mock_env(),
            MessageInfo {
                sender: Addr::unchecked(sender),
                funds: vec![],
            },
            ExecuteMsg::RegisterAgent {
                payable_account_id: Some(Addr::unchecked(AGENT_BENEFICIARY)),
            },
        )
    }

    fn get_stored_agent_status(app: &mut App, contract_addr: &Addr, agent: &str) -> AgentStatus {
        let agent_info: AgentResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetAgent {
                    account_id: Addr::unchecked(agent),
                },
            )
            .expect("Error getting agent status");
        agent_info.status
    }

    fn register_agent_exec(
        app: &mut App,
        contract_addr: &Addr,
        agent: &str,
        beneficiary: &str,
    ) -> AppResponse {
        app.execute_contract(
            Addr::unchecked(agent),
            contract_addr.clone(),
            &ExecuteMsg::RegisterAgent {
                payable_account_id: Some(Addr::unchecked(beneficiary)),
            },
            &[],
        )
        .expect("Error registering agent")
    }

    fn check_in_exec(
        app: &mut App,
        contract_addr: &Addr,
        agent: &str,
    ) -> Result<AppResponse, anyhow::Error> {
        app.execute_contract(
            Addr::unchecked(agent),
            contract_addr.clone(),
            &ExecuteMsg::CheckInAgent {},
            &[],
        )
    }

    fn get_agent_ids(app: &App, contract_addr: &Addr) -> (GetAgentIdsResponse, usize, usize) {
        let res: GetAgentIdsResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::GetAgentIds {})
            .unwrap();
        (res.clone(), res.active.len(), res.pending.len())
    }

    pub fn add_little_time(block: &mut BlockInfo) {
        // block.time = block.time.plus_seconds(360);
        block.time = block.time.plus_seconds(19);
        block.height += 1;
    }

    pub fn add_one_duration_of_time(block: &mut BlockInfo) {
        // block.time = block.time.plus_seconds(360);
        block.time = block.time.plus_seconds(420);
        block.height += 1;
    }

    #[test]
    fn register_agent_fail_cases() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        // start first register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT_BENEFICIARY)),
        };

        // Test funds fail register if sent
        let rereg_err = app
            .execute_contract(
                Addr::unchecked(AGENT1),
                contract_addr.clone(),
                &msg,
                &coins(37, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Do not attach funds".to_string()
            },
            rereg_err.downcast().unwrap()
        );

        // Test Can't register if contract is paused
        let payload_1 = ExecuteMsg::UpdateSettings {
            paused: Some(true),
            owner_id: None,
            // treasury_id: None,
            agent_fee: None,
            min_tasks_per_agent: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
        };

        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &payload_1,
            &[],
        )
        .unwrap();
        let rereg_err = app
            .execute_contract(Addr::unchecked(AGENT1), contract_addr.clone(), &msg, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::ContractPaused {
                val: "Register agent paused".to_string()
            },
            rereg_err.downcast().unwrap()
        );

        // Test wallet rejected if doesnt have enough funds
        let payload_2 = ExecuteMsg::UpdateSettings {
            paused: Some(false),
            owner_id: None,
            // treasury_id: None,
            agent_fee: None,
            min_tasks_per_agent: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
        };

        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &payload_2,
            &[],
        )
        .unwrap();
        let rereg_err = app
            .execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Insufficient funds".to_string()
            },
            rereg_err.downcast().unwrap()
        );
    }

    #[test]
    fn register_agent() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let blk_time = app.block_info().time;

        // start first register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT1), contract_addr.clone(), &msg, &[])
            .unwrap();

        // check state to see if worked
        let (_, num_active_agents, num_pending_agents) = get_agent_ids(&app, &contract_addr);
        assert_eq!(1, num_active_agents);
        assert_eq!(0, num_pending_agents);

        // message response matches expectations (same block, all the defaults)
        let agent_info: AgentResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetAgent {
                    account_id: Addr::unchecked(AGENT1),
                },
            )
            .unwrap();
        println!("agent_infoagent_info {:?}", agent_info);
        assert_eq!(AgentStatus::Active, agent_info.status);
        assert_eq!(
            Addr::unchecked(AGENT_BENEFICIARY),
            agent_info.payable_account_id
        );
        assert_eq!(GenericBalance::default(), agent_info.balance);
        assert_eq!(0, agent_info.total_tasks_executed);
        assert_eq!(0, agent_info.last_missed_slot);
        assert_eq!(blk_time, agent_info.register_start);

        // test fail if try to re-register
        let rereg_err = app
            .execute_contract(Addr::unchecked(AGENT1), contract_addr.clone(), &msg, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Agent already exists".to_string()
            },
            rereg_err.downcast().unwrap()
        );

        // test another register, put into pending queue
        let msg2 = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT2), contract_addr.clone(), &msg2, &[])
            .unwrap();

        // check state to see if worked

        let (_, num_active_agents, num_pending_agents) = get_agent_ids(&app, &contract_addr);
        assert_eq!(1, num_active_agents);
        assert_eq!(1, num_pending_agents);
    }

    #[test]
    fn update_agent() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        // start first register
        let msg1 = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT1), contract_addr.clone(), &msg1, &[])
            .unwrap();

        // Fails for non-existent agents
        let msg = ExecuteMsg::UpdateAgent {
            payable_account_id: Addr::unchecked(AGENT0),
        };
        let update_err = app
            .execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::AgentNotRegistered {},
            update_err.downcast().unwrap()
        );

        app.execute_contract(Addr::unchecked(AGENT1), contract_addr.clone(), &msg, &[])
            .unwrap();

        // payable account was in fact updated
        let agent_info: Agent = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetAgent {
                    account_id: Addr::unchecked(AGENT1),
                },
            )
            .unwrap();
        assert_eq!(Addr::unchecked(AGENT0), agent_info.payable_account_id);
    }

    #[test]
    fn unregister_agent() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        // start first register
        let msg1 = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT1), contract_addr.clone(), &msg1, &[])
            .unwrap();

        // Fails for non-exist agents
        let unreg_msg = ExecuteMsg::UnregisterAgent {};
        let update_err = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &unreg_msg,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::AgentNotRegistered {},
            update_err.downcast().unwrap()
        );

        // Get quick data about account before, to compare later
        let agent_bal = app
            .wrap()
            .query_balance(&Addr::unchecked(AGENT1), NATIVE_DENOM)
            .unwrap();
        assert_eq!(agent_bal, coin(2_000_000, NATIVE_DENOM));

        // Attempt the unregister
        app.execute_contract(
            Addr::unchecked(AGENT1),
            contract_addr.clone(),
            &unreg_msg,
            &[],
        )
        .unwrap();

        // Agent should not exist now
        let update_err = app
            .execute_contract(
                Addr::unchecked(AGENT1),
                contract_addr.clone(),
                &unreg_msg,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::AgentNotRegistered {},
            update_err.downcast().unwrap()
        );

        // Check that the agent was removed from the list of active or pending agents
        let (_, num_active_agents, num_pending_agents) = get_agent_ids(&app, &contract_addr);
        assert_eq!(0, num_active_agents);
        assert_eq!(0, num_pending_agents);

        // Agent should have appropriate balance change
        // NOTE: Needs further checks when tasks can be performed
        let agent_bal = app
            .wrap()
            .query_balance(&Addr::unchecked(AGENT1), NATIVE_DENOM)
            .unwrap();
        assert_eq!(agent_bal, coin(2000000, NATIVE_DENOM));
    }

    #[test]
    fn withdraw_agent_balance() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        // start first register
        let msg1 = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT1), contract_addr.clone(), &msg1, &[])
            .unwrap();

        // Fails for non-existent agents
        let wthdrw_msg = ExecuteMsg::WithdrawReward {};
        let update_err = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &wthdrw_msg,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::AgentNotRegistered {},
            update_err.downcast().unwrap()
        );

        // Get quick data about account before, to compare later
        let agent_bal = app
            .wrap()
            .query_balance(&Addr::unchecked(AGENT1), NATIVE_DENOM)
            .unwrap();
        assert_eq!(agent_bal, coin(2_000_000, NATIVE_DENOM));

        // Attempt the withdraw
        app.execute_contract(
            Addr::unchecked(AGENT1),
            contract_addr.clone(),
            &wthdrw_msg,
            &[],
        )
        .unwrap();

        // Agent should have appropriate balance change
        // NOTE: Needs further checks when tasks can be performed
        let agent_bal = app
            .wrap()
            .query_balance(&Addr::unchecked(AGENT1), NATIVE_DENOM)
            .unwrap();
        assert_eq!(agent_bal, coin(2_000_000, NATIVE_DENOM));
    }

    #[test]
    fn accept_nomination_agent() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        // Register AGENT1, who immediately becomes active
        register_agent_exec(&mut app, &contract_addr, AGENT1, &AGENT_BENEFICIARY);
        let res = add_task_exec(&mut app, &contract_addr, PARTICIPANT0);
        let task_hash = res.events[1].attributes[4].clone().value;
        assert_eq!(
            "7ea9a6d5ef5c78cb168afa96b43b5843b8f880627aa0580f4311403f907cbf93", task_hash,
            "Unexpected task hash"
        );

        let msg_query_task = QueryMsg::GetTask { task_hash };
        let query_task_res: StdResult<Option<TaskResponse>> = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &msg_query_task);
        assert!(
            query_task_res.is_ok(),
            "Did not successfully find the newly added task"
        );

        let mut num_tasks = get_task_total(&app, &contract_addr);
        assert_eq!(num_tasks, 1);

        // Now the task ratio is 1:2 (one agent per two tasks)
        // No agent should be allowed to join or accept nomination
        // Check that this fails

        // Register two agents
        register_agent_exec(&mut app, &contract_addr, AGENT2, &AGENT_BENEFICIARY);
        // Later, we'll have this agent try to nominate themselves before their time
        register_agent_exec(&mut app, &contract_addr, AGENT3, &AGENT_BENEFICIARY);

        let (agent_ids_res, num_active_agents, _) = get_agent_ids(&app, &contract_addr);
        assert_eq!(1, num_active_agents);
        assert_eq!(2, agent_ids_res.pending.len());

        // Add three more tasks, so we can nominate another agent
        add_task_exec(&mut app, &contract_addr, PARTICIPANT1);
        add_task_exec(&mut app, &contract_addr, PARTICIPANT2);
        add_task_exec(&mut app, &contract_addr, PARTICIPANT3);

        num_tasks = get_task_total(&app, &contract_addr);
        assert_eq!(num_tasks, 4);

        // Fast forward time a little
        app.update_block(add_little_time);

        let mut agent_status = get_stored_agent_status(&mut app, &contract_addr, AGENT3);
        assert_eq!(AgentStatus::Pending, agent_status);
        agent_status = get_stored_agent_status(&mut app, &contract_addr, AGENT2);
        assert_eq!(AgentStatus::Nominated, agent_status);

        // Attempt to accept nomination
        // First try with the agent second in line in the pending queue.
        // This should fail because it's not time for them yet.
        let mut check_in_res = check_in_exec(&mut app, &contract_addr, AGENT3);
        assert!(
            &check_in_res.is_err(),
            "Should throw error when agent in second position tries to nominate before their time."
        );
        assert_eq!(
            ContractError::CustomError {
                val: "Must wait longer before accepting nomination".to_string()
            },
            check_in_res.unwrap_err().downcast().unwrap()
        );

        // Now try from person at the beginning of the pending queue
        // This agent should succeed
        check_in_res = check_in_exec(&mut app, &contract_addr, AGENT2);
        assert!(
            check_in_res.is_ok(),
            "Agent at the front of the pending queue should be allowed to nominate themselves"
        );

        // Check that active and pending queues are correct
        let (agent_ids_res, num_active_agents, _) = get_agent_ids(&app, &contract_addr);
        assert_eq!(2, num_active_agents);
        assert_eq!(1, agent_ids_res.pending.len());

        // The agent that was second in the queue is now first,
        // tries again, but there aren't enough tasks
        check_in_res = check_in_exec(&mut app, &contract_addr, AGENT3);

        let error_msg = check_in_res.unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Not accepting new agents".to_string()
            },
            error_msg.downcast().unwrap()
        );

        agent_status = get_stored_agent_status(&mut app, &contract_addr, AGENT3);
        assert_eq!(AgentStatus::Pending, agent_status);

        // Again, add three more tasks so we can nominate another agent
        add_task_exec(&mut app, &contract_addr, PARTICIPANT4);
        add_task_exec(&mut app, &contract_addr, PARTICIPANT5);
        add_task_exec(&mut app, &contract_addr, PARTICIPANT6);

        num_tasks = get_task_total(&app, &contract_addr);
        assert_eq!(num_tasks, 7);

        // Add another agent, since there's now the need
        register_agent_exec(&mut app, &contract_addr, AGENT4, &AGENT_BENEFICIARY);
        // Fast forward time past the duration of the first pending agent,
        // allowing the second to nominate themselves
        app.update_block(add_one_duration_of_time);

        // Now that enough time has passed, both agents should see they're nominated
        agent_status = get_stored_agent_status(&mut app, &contract_addr, AGENT3);
        assert_eq!(AgentStatus::Nominated, agent_status);
        agent_status = get_stored_agent_status(&mut app, &contract_addr, AGENT4);
        assert_eq!(AgentStatus::Nominated, agent_status);

        // Agent second in line nominates themself
        check_in_res = check_in_exec(&mut app, &contract_addr, AGENT4);
        assert!(
            check_in_res.is_ok(),
            "Agent second in line should be able to nominate themselves"
        );

        let (_, _, num_pending_agents) = get_agent_ids(&app, &contract_addr);

        // Ensure the pending list is empty, having the earlier index booted
        assert_eq!(
            num_pending_agents, 0,
            "Expect the pending queue to be empty"
        );
    }

    #[test]
    fn test_get_agent_status() {
        // Give the contract and the agents balances
        let mut deps = cosmwasm_std::testing::mock_dependencies_with_balances(&[
            (&MOCK_CONTRACT_ADDR, &[coin(6000, "atom")]),
            (&AGENT0, &[coin(2_000_000, "atom")]),
            (&AGENT1, &[coin(2_000_000, "atom")]),
        ]);
        let mut contract = CwCroncat::default();

        // Instantiate
        let msg = InstantiateMsg {
            denom: "atom".to_string(),
            owner_id: None,
            gas_base_fee: None,
            agent_nomination_duration: Some(360),
        };
        let mut info = mock_info(AGENT0, &coins(900_000, "atom"));
        let res_init = contract
            .instantiate(deps.as_mut(), mock_env(), info.clone(), msg)
            .unwrap();
        assert_eq!(0, res_init.messages.len());

        let mut agent_status_res =
            contract.get_agent_status(&deps.storage, mock_env(), Addr::unchecked(AGENT0));
        assert_eq!(Err(ContractError::AgentNotRegistered {}), agent_status_res);

        let agent_active_queue_opt: Vec<Addr> =
            match deps.storage.get("agent_active_queue".as_bytes()) {
                Some(vec) => from_slice(vec.as_ref()).expect("Could not load agent active queue"),
                None => {
                    panic!("Uninitialized agent_active_queue_opt");
                }
            };
        assert!(
            agent_active_queue_opt.is_empty(),
            "Should not have an active queue yet"
        );

        // First registered agent becomes active
        let mut register_agent_res = contract_register_agent(AGENT0, &mut contract, deps.as_mut());
        assert!(
            register_agent_res.is_ok(),
            "Registering agent should succeed"
        );

        agent_status_res =
            contract.get_agent_status(&deps.storage, mock_env(), Addr::unchecked(AGENT0));
        assert_eq!(AgentStatus::Active, agent_status_res.unwrap());

        // Add two tasks
        let mut res_add_task = contract_create_task(&contract, deps.as_mut(), &info);
        assert!(res_add_task.is_ok(), "Adding task should succeed.");
        // Change sender so it's not a duplicate task
        info.sender = Addr::unchecked(PARTICIPANT0);
        res_add_task = contract_create_task(&contract, deps.as_mut(), &info);
        assert!(res_add_task.is_ok(), "Adding task should succeed.");

        // Register an agent and make sure the status comes back as pending
        register_agent_res = contract_register_agent(AGENT1, &mut contract, deps.as_mut());
        assert!(
            register_agent_res.is_ok(),
            "Registering agent should succeed"
        );
        agent_status_res =
            contract.get_agent_status(&deps.storage, mock_env(), Addr::unchecked(AGENT1));
        assert_eq!(
            AgentStatus::Pending,
            agent_status_res.unwrap(),
            "New agent should be pending"
        );

        // Two more tasks are added
        info.sender = Addr::unchecked(PARTICIPANT1);
        res_add_task = contract_create_task(&contract, deps.as_mut(), &info);
        assert!(res_add_task.is_ok(), "Adding task should succeed.");
        info.sender = Addr::unchecked(PARTICIPANT2);
        res_add_task = contract_create_task(&contract, deps.as_mut(), &info);
        assert!(res_add_task.is_ok(), "Adding task should succeed.");

        // Agent status is nominated
        agent_status_res =
            contract.get_agent_status(&deps.storage, mock_env(), Addr::unchecked(AGENT1));
        assert_eq!(
            AgentStatus::Nominated,
            agent_status_res.unwrap(),
            "New agent should have nominated status"
        );
    }

    #[test]
    fn test_query_get_agent_tasks() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let block_info = app.block_info();
        println!(
            "test aloha\n\tcurrent block: {}\n\tcurrent time: {}",
            block_info.height,
            block_info.time.nanos()
        );

        // Register AGENT1, who immediately becomes active
        register_agent_exec(&mut app, &contract_addr, AGENT1, &AGENT_BENEFICIARY);
        // Add five tasks total
        // Three of them are block-based
        add_block_task_exec(
            &mut app,
            &contract_addr,
            PARTICIPANT0,
            block_info.height + 6,
        );
        add_block_task_exec(
            &mut app,
            &contract_addr,
            PARTICIPANT1,
            block_info.height + 66,
        );
        add_block_task_exec(
            &mut app,
            &contract_addr,
            PARTICIPANT2,
            block_info.height + 67,
        );
        // add_block_task_exec(&mut app, &contract_addr, PARTICIPANT3, block_info.height + 131);
        // Two tasks use Cron instead of Block (for task interval)
        add_cron_task_exec(&mut app, &contract_addr, PARTICIPANT4, 6); // 3 minutes
        add_cron_task_exec(&mut app, &contract_addr, PARTICIPANT5, 53); // 53 minutes
        let num_tasks = get_task_total(&app, &contract_addr);
        assert_eq!(num_tasks, 5);

        // Now the task ratio is 1:2 (one agent per two tasks)
        // Register two agents, the first one succeeding
        register_agent_exec(&mut app, &contract_addr, AGENT2, &AGENT_BENEFICIARY);
        assert!(check_in_exec(&mut app, &contract_addr, AGENT2).is_ok());
        // This next agent should fail because there's no enough tasks yet
        // Later, we'll have this agent try to nominate themselves before their time
        register_agent_exec(&mut app, &contract_addr, AGENT3, &AGENT_BENEFICIARY);
        let failed_check_in = check_in_exec(&mut app, &contract_addr, AGENT3);
        assert_eq!(
            ContractError::CustomError {
                val: "Not accepting new agents".to_string()
            },
            failed_check_in.unwrap_err().downcast().unwrap()
        );

        let (_, num_active_agents, num_pending_agents) = get_agent_ids(&app, &contract_addr);
        assert_eq!(2, num_active_agents);
        assert_eq!(1, num_pending_agents);

        // Fast forward time a little
        app.update_block(|block| {
            let height = 666;
            block.time = block.time.plus_seconds(6 * height); // ~6 sec block time
            block.height = block.height + height;
        });

        // What happens when the only active agent queries to see if there's work for them
        // calls:
        // fn query_get_agent_tasks
        let mut msg_agent_tasks = QueryMsg::GetAgentTasks {
            account_id: Addr::unchecked(AGENT1),
        };
        let mut query_task_res: StdResult<Option<AgentTaskResponse>> = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &msg_agent_tasks);
        println!(
            "test aloha query_task_res0 {:#?}",
            query_task_res.as_ref().unwrap()
        );
        assert!(
            query_task_res.is_ok(),
            "Did not successfully find the newly added task"
        );
        msg_agent_tasks = QueryMsg::GetAgentTasks {
            account_id: Addr::unchecked(AGENT2),
        };
        query_task_res = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &msg_agent_tasks);
        println!("test aloha query_task_res1 {:#?}", query_task_res.unwrap());
        // Should fail for random user not in the active queue
        msg_agent_tasks = QueryMsg::GetAgentTasks {
            // rando account
            account_id: Addr::unchecked("juno1kqfjv53g7ll9u6ngvsu5l5nfv9ht24m4q4gdqz"),
        };
        query_task_res = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &msg_agent_tasks);
        println!("aloha query_task_res {:?}", query_task_res);
    }
}
