use crate::helpers::*;
use crate::state::{Config, TaskInfo};
use crate::ContractError::AgentNotRegistered;
use cosmwasm_std::Storage;
use cosmwasm_std::{Addr, Env, StdError, StdResult};
use cosmwasm_std::{DepsMut, Uint64};
use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::types::SlotType;
use cw_storage_plus::Item;

#[derive(PartialEq, Eq)]
pub enum BalancerMode {
    ActivationOrder,
    Equalizer,
}
pub trait Balancer<'a> {
    fn get_agent_tasks(
        &mut self,
        deps: &DepsMut,
        env: &Env,
        config: &Item<'a, Config>,
        active_agents: &Item<'a, Vec<Addr>>,
        agent_id: Addr,
        slot_items: (Option<u64>, Option<u64>),
    ) -> StdResult<Option<AgentTaskResponse>>;
    fn on_agent_unregister(
        &self,
        storage: &'a mut dyn Storage,
        config: &Item<'a, Config>,
        active_agents: &Item<'a, Vec<Addr>>,
        agent_id: Addr,
    );
    fn on_task_completed(
        &self,
        storage: &'a mut dyn Storage,
        _env: &Env,
        config: &Item<'a, Config>,
        active_agents: &Item<'a, Vec<Addr>>,
        task_info: TaskInfo,
    );
}

pub struct RoundRobinBalancer {
    pub mode: BalancerMode,
}

impl RoundRobinBalancer {
    pub fn default() -> RoundRobinBalancer {
        RoundRobinBalancer::new(BalancerMode::ActivationOrder)
    }
    pub fn new(mode: BalancerMode) -> RoundRobinBalancer {
        RoundRobinBalancer { mode }
    }
    fn update_or_append(
        &self,
        overflows: &mut Vec<(SlotType, u32, u32)>,
        value: (SlotType, u32, u32),
    ) {
        match overflows
            .iter_mut()
            .find(|p| p.0 == value.0 && p.1 == value.1)
        {
            Some(found) => {
                found.2 += value.2;
            }
            None => {
                overflows.push(value);
            }
        }
    }
    fn remove_agent_and_rebalance(
        &self,
        indices: &mut Vec<(SlotType, u32, u32)>,
        agent_index: u32,
    ) {
        indices.clear();
        let mut vec: Vec<(SlotType, u32, u32)> = Vec::new();
        for p in indices.iter() {
            match agent_index {
                aind if aind < p.1 => vec.push((p.0.clone(), p.1 - 1, p.2)),
                aind if aind > p.1 => vec.push((p.0.clone(), p.1, p.2)),
                _ => (),
            }
        }
        indices.extend(vec);
    }
}
impl<'a> Balancer<'a> for RoundRobinBalancer {
    fn get_agent_tasks(
        &mut self,
        deps: &DepsMut,
        _env: &Env,
        config: &Item<'a, Config>,
        active_agents: &Item<'a, Vec<Addr>>,
        agent_id: Addr,
        slot_items: (Option<u64>, Option<u64>),
    ) -> StdResult<Option<AgentTaskResponse>> {
        let conf: Config = config.load(deps.storage)?;
        let active = active_agents.load(deps.storage)?;
        if !active.contains(&agent_id) {
            return Err(StdError::GenericErr {
                msg: AgentNotRegistered {}.to_string(),
            });
        }
        let agent_count = active.len() as u64;
        let agent_active_indices_config = conf.agent_active_indices;
        let agent_active_indices: Vec<usize> = (0..active.len()).collect();
        let agent_index = active
            .iter()
            .position(|x| x == &agent_id)
            .expect("Agent is not active or not registered!") as u64;

        if slot_items == (None, None) {
            return Ok(None);
        }
        let mut num_block_tasks = Uint64::from(0u64);
        let mut num_block_tasks_extra = Uint64::from(0u64);

        let mut num_cron_tasks = Uint64::from(0u64);
        let mut num_cron_tasks_extra = Uint64::from(0u64);

        match self.mode {
            BalancerMode::ActivationOrder => {
                let activation_ordering = |total_tasks: u64| -> (Uint64, Uint64) {
                    if total_tasks < 1 {
                        return (Uint64::from(0u64), Uint64::from(0u64));
                    }
                    if total_tasks <= active.len() as u64 {
                        let agent_tasks_total = 1u64.saturating_sub(
                            agent_index.saturating_sub(total_tasks.saturating_sub(1)),
                        );
                        (agent_tasks_total.into(), agent_tasks_total.into())
                    } else {
                        let leftover = total_tasks % agent_count;

                        let mut rich_agents: Vec<(SlotType, u32, u32)> =
                            agent_active_indices_config
                                .clone()
                                .into_iter()
                                .filter(|e| e.2 > 0)
                                .collect::<_>();

                        rich_agents.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
                        let rich_indices: Vec<usize> =
                            rich_agents.iter().map(|v| v.1 as usize).collect();

                        let mut diff = vect_difference(&agent_active_indices, &rich_indices);
                        diff.extend(rich_indices);
                        let agent_index = diff
                            .iter()
                            .position(|x| x == &(agent_index as usize))
                            .expect("Agent is not active or not registered!")
                            as u64;

                        let mut extra = 0u64;
                        if leftover > 0 {
                            extra = 1u64.saturating_sub(
                                agent_index.saturating_sub(leftover.saturating_sub(1)),
                            );
                        }
                        let agent_tasks_total = total_tasks.saturating_div(agent_count) + extra;

                        (agent_tasks_total.into(), extra.into())
                    }
                };

                if let Some(current_block_task_total) = slot_items.0 {
                    let (n, ne) = activation_ordering(current_block_task_total);
                    num_block_tasks = n;
                    num_block_tasks_extra = ne;
                }
                if let Some(current_cron_task_total) = slot_items.1 {
                    let (n, ne) = activation_ordering(current_cron_task_total);
                    num_cron_tasks = n;
                    num_cron_tasks_extra = ne;
                }

                Ok(Some(AgentTaskResponse {
                    num_block_tasks,
                    num_block_tasks_extra,
                    num_cron_tasks,
                    num_cron_tasks_extra,
                }))
            }
            BalancerMode::Equalizer => todo!(),
        }
    }

    fn on_agent_unregister(
        &self,
        storage: &'a mut dyn Storage,
        config: &Item<'a, Config>,
        active_agents: &Item<'a, Vec<Addr>>,
        agent_id: Addr,
    ) {
        let mut conf: Config = config.load(storage).unwrap();
        let indices = conf.agent_active_indices.as_mut();
        let active = active_agents.load(storage).unwrap();
        let agent_index = active
            .iter()
            .position(|x| x == &agent_id)
            .expect("Agent is not active or not registered!") as u32;

        self.remove_agent_and_rebalance(indices, agent_index);

        config.save(storage, &conf).unwrap();
    }

    fn on_task_completed(
        &self,
        storage: &'a mut dyn Storage,
        _env: &Env,
        config: &Item<'a, Config>,
        active_agents: &Item<'a, Vec<Addr>>,
        task_info: TaskInfo,
    ) {
        if !task_info.task_is_extra.unwrap_or(false) {
            return;
        };

        let mut conf: Config = config.load(storage).unwrap();
        let indices = conf.agent_active_indices.as_mut();
        let active = active_agents.load(storage).unwrap();
        let agent_id = task_info.agent_id.unwrap();
        let slot_kind = task_info.slot_kind;
        let agent_index = active
            .iter()
            .position(|x| x == &agent_id)
            .expect("Agent is not active or not registered!") as u32;

        self.update_or_append(indices, (slot_kind, agent_index, 1));
        config.save(storage, &conf).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::GAS_BASE_FEE_JUNO;
    use crate::error::ContractError;
    use crate::helpers::CwTemplateContract;
    use cosmwasm_std::testing::{
        mock_dependencies_with_balance, mock_env, mock_info, MockStorage, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{
        coin, coins, from_slice, Addr, BlockInfo, Coin, CosmosMsg, Empty, StakingMsg,
    };
    use cw_croncat_core::types::{Agent, SlotType, Task};

    use cw_croncat_core::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TaskRequest, TaskResponse};
    use cw_croncat_core::types::{Action, Boundary, Interval};
    use cw_multi_test::{App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor};

    use crate::CwCroncat;
    const AGENT0: &str = "cosmos1a7uhnpqthunr2rzj0ww0hwurpn42wyun6c5puz";
    const AGENT1: &str = "cosmos17muvdgkep4ndptnyg38eufxsssq8jr3wnkysy8";
    const AGENT2: &str = "cosmos1qxywje86amll9ptzxmla5ah52uvsd9f7drs2dl";
    const AGENT3: &str = "cosmos1c3cy3wzzz3698ypklvh7shksvmefj69xhm89z2";
    const AGENT4: &str = "cosmos1ykfcyj8fl6xzs88tsls05x93gmq68a7km05m4j";
    const ADMIN: &str = "cosmos1sjllsnramtg3ewxqwwrwjxfgc4n4ef9u0tvx7u";
    const NATIVE_DENOM: &str = "atom";

    fn mock_config() -> Config {
        Config {
            paused: false,
            owner_id: Addr::unchecked(ADMIN),
            // treasury_id: None,
            min_tasks_per_agent: 3,
            agent_active_indices: Vec::<(SlotType, u32, u32)>::with_capacity(0),
            agents_eject_threshold: 600, // how many slots an agent can miss before being ejected. 10 * 60 = 1hr
            available_balance: GenericBalance::default(),
            staked_balance: GenericBalance::default(),
            agent_fee: Coin::new(5, NATIVE_DENOM.clone()), // TODO: CHANGE AMOUNT HERE!!! 0.0005 Juno (2000 tasks = 1 Juno)
            gas_price: 1,
            gas_base_fee: GAS_BASE_FEE_JUNO,
            proxy_callback_gas: 3,
            slot_granularity: 60_000_000_000,
            native_denom: NATIVE_DENOM.to_owned(),
            cw20_whitelist: vec![],
            agent_nomination_duration: 9,
        }
    }
    #[test]
    fn test_agent_has_valid_task_count() {
        let store = CwCroncat::default();
        let mut deps = mock_dependencies_with_balance(&coins(200, NATIVE_DENOM));
        let env = mock_env();
        let mut balancer = RoundRobinBalancer::default();
        let config = mock_config();

        store.config.save(&mut deps.storage, &config).unwrap();

        let mut active_agents: Vec<Addr> = store
            .agent_active_queue
            .may_load(&deps.storage)
            .unwrap()
            .unwrap_or_default();
        active_agents.extend(vec![
            Addr::unchecked(AGENT0),
            Addr::unchecked(AGENT1),
            Addr::unchecked(AGENT2),
            Addr::unchecked(AGENT3),
            Addr::unchecked(AGENT4),
        ]);

        store
            .agent_active_queue
            .save(&mut deps.storage, &active_agents)
            .unwrap();
        let slot: (Option<u64>, Option<u64>) = (Some(1), Some(2));
        let result = balancer
            .get_agent_tasks(
                &deps.as_mut(),
                &env.clone(),
                &store.config,
                &store.agent_active_queue,
                Addr::unchecked(AGENT0),
                slot,
            )
            .unwrap()
            .unwrap();
        assert_eq!(result.num_block_tasks.u64(), 1);
        assert_eq!(result.num_cron_tasks.u64(), 1);

        //Verify earch gents valid amount
        let slot: (Option<u64>, Option<u64>) = (Some(100), Some(50));
        let result = balancer
            .get_agent_tasks(
                &deps.as_mut(),
                &env.clone(),
                &store.config,
                &store.agent_active_queue,
                Addr::unchecked(AGENT0),
                slot,
            )
            .unwrap()
            .unwrap();
        assert!(result.num_block_tasks.u64() == 20);
        assert!(result.num_cron_tasks.u64() == 10);

        //Verify agents gets zero
        let slot: (Option<u64>, Option<u64>) = (Some(0), Some(0));
        let result = balancer
            .get_agent_tasks(
                &deps.as_mut(),
                &env.clone(),
                &store.config,
                &store.agent_active_queue,
                Addr::unchecked(AGENT0),
                slot,
            )
            .unwrap()
            .unwrap();
        assert!(result.num_block_tasks.u64() == 0);
        assert!(result.num_cron_tasks.u64() == 0);
    }

    #[test]
    fn test_check_valid_agents_get_extra_tasks() {
        let store = CwCroncat::default();
        let mut deps = mock_dependencies_with_balance(&coins(200, NATIVE_DENOM));
        let env = mock_env();
        let mut balancer = RoundRobinBalancer::default();
        let config = mock_config();

        store.config.save(&mut deps.storage, &config).unwrap();

        let mut active_agents: Vec<Addr> = store
            .agent_active_queue
            .may_load(&deps.storage)
            .unwrap()
            .unwrap_or_default();
        active_agents.extend(vec![
            Addr::unchecked(AGENT0),
            Addr::unchecked(AGENT1),
            Addr::unchecked(AGENT2),
            Addr::unchecked(AGENT3),
            Addr::unchecked(AGENT4),
        ]);

        store
            .agent_active_queue
            .save(&mut deps.storage, &active_agents)
            .unwrap();

        //Verify agent0 gets extra
        let slot: (Option<u64>, Option<u64>) = (Some(7), Some(7));
        let result = balancer
            .get_agent_tasks(
                &deps.as_mut(),
                &env.clone(),
                &store.config,
                &store.agent_active_queue,
                Addr::unchecked(AGENT0),
                slot,
            )
            .unwrap()
            .unwrap();

        assert_eq!(result.num_block_tasks.u64(), 2);
        assert_eq!(result.num_cron_tasks.u64(), 2);
        assert_eq!(result.num_block_tasks_extra.u64(), 1);
        assert_eq!(result.num_cron_tasks_extra.u64(), 1);

        //Verify agent1 gets extra
        let result = balancer
            .get_agent_tasks(
                &deps.as_mut(),
                &env.clone(),
                &store.config,
                &store.agent_active_queue,
                Addr::unchecked(AGENT1),
                slot,
            )
            .unwrap()
            .unwrap();

        assert_eq!(result.num_block_tasks.u64(), 2);
        assert_eq!(result.num_cron_tasks.u64(), 2);
        assert_eq!(result.num_block_tasks_extra.u64(), 1);
        assert_eq!(result.num_cron_tasks_extra.u64(), 1);

        //Verify agent3 not getting extra
        let result = balancer
            .get_agent_tasks(
                &deps.as_mut(),
                &env.clone(),
                &store.config,
                &store.agent_active_queue,
                Addr::unchecked(AGENT3),
                slot,
            )
            .unwrap()
            .unwrap();

        assert_eq!(result.num_block_tasks.u64(), 1);
        assert_eq!(result.num_cron_tasks.u64(), 1);
        assert_eq!(result.num_block_tasks_extra.u64(), 0);
        assert_eq!(result.num_cron_tasks_extra.u64(), 0);
    }
    #[test]
    fn test_on_task_completed() {
        let store = CwCroncat::default();
        let mut deps = mock_dependencies_with_balance(&coins(200, NATIVE_DENOM));
        let env = mock_env();
        let balancer = RoundRobinBalancer::default();
        let mut config = mock_config();

        store.config.save(&mut deps.storage, &config).unwrap();

        let mut active_agents: Vec<Addr> = store
            .agent_active_queue
            .may_load(&deps.storage)
            .unwrap()
            .unwrap_or_default();
        active_agents.extend(vec![
            Addr::unchecked(AGENT0),
            Addr::unchecked(AGENT1),
            Addr::unchecked(AGENT2),
            Addr::unchecked(AGENT3),
            Addr::unchecked(AGENT4),
        ]);

        store
            .agent_active_queue
            .save(&mut deps.storage, &active_agents)
            .unwrap();

        let task_info = TaskInfo {
            task: None,
            task_hash: "".as_bytes().to_vec(),
            task_is_extra: Some(true),
            agent_id: Some(Addr::unchecked(AGENT0)),
            slot_kind: SlotType::Block,
        };

        balancer.update_or_append(&mut config.agent_active_indices, (SlotType::Block, 0, 10));
        store.config.save(&mut deps.storage, &config).unwrap();
        balancer.on_task_completed(
            &mut deps.storage,
            &env,
            &store.config,
            &store.agent_active_queue,
            task_info,
        );

        config = store.config.load(&mut deps.storage).unwrap();
        assert_eq!(config.agent_active_indices, vec![(SlotType::Block, 0, 11)])
    }

    #[test]
    fn test_on_agent_unregister() {
        let store = CwCroncat::default();
        let mut deps = mock_dependencies_with_balance(&coins(200, NATIVE_DENOM));
        let balancer = RoundRobinBalancer::default();
        let mut config = mock_config();

        store.config.save(&mut deps.storage, &config).unwrap();

        let mut active_agents: Vec<Addr> = store
            .agent_active_queue
            .may_load(&deps.storage)
            .unwrap()
            .unwrap_or_default();
        active_agents.extend(vec![
            Addr::unchecked(AGENT0),
            Addr::unchecked(AGENT1),
            Addr::unchecked(AGENT2),
            Addr::unchecked(AGENT3),
            Addr::unchecked(AGENT4),
        ]);

        store
            .agent_active_queue
            .save(&mut deps.storage, &active_agents)
            .unwrap();

        balancer.update_or_append(&mut config.agent_active_indices, (SlotType::Block, 0, 1));
        balancer.update_or_append(&mut config.agent_active_indices, (SlotType::Cron, 0, 1));
        store.config.save(&mut deps.storage, &config).unwrap();
        balancer.on_agent_unregister(
            &mut deps.storage,
            &store.config,
            &store.agent_active_queue,
            Addr::unchecked(AGENT0),
        );

        config = store.config.load(&mut deps.storage).unwrap();
        assert_eq!(config.agent_active_indices, vec![])
    }
}
