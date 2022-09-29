use crate::balancer::{Balancer, BalancerMode, RoundRobinBalancer};
use crate::contract::GAS_BASE_FEE_JUNO;
use crate::state::{Config, TaskInfo};
use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env};
use cosmwasm_std::{coins, Addr};
use cw_croncat_core::types::{GenericBalance, SlotType};

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
        agent_fee: 5,
        gas_price: 1,
        gas_base_fee: GAS_BASE_FEE_JUNO,
        proxy_callback_gas: 3,
        slot_granularity: 60_000_000_000,
        native_denom: NATIVE_DENOM.to_owned(),
        cw20_whitelist: vec![],
        agent_nomination_duration: 9,
        limit: 100,
        cw_rules_addr: Addr::unchecked("todo"),
    }
}
#[test]
fn test_agent_has_valid_task_count_ao_mode() {
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
            &deps.as_ref(),
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
            &deps.as_ref(),
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
            &deps.as_ref(),
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
fn test_check_valid_agents_get_extra_tasks_ao_mode() {
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
            &deps.as_ref(),
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
            &deps.as_ref(),
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
            &deps.as_ref(),
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
fn test_check_valid_agents_get_extra_tasks_eq_mode() {
    let store = CwCroncat::default();
    let mut deps = mock_dependencies_with_balance(&coins(200, NATIVE_DENOM));
    let env = mock_env();
    let mut balancer = RoundRobinBalancer::new(BalancerMode::Equalizer);
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

    let task_info = TaskInfo {
        task: None,
        task_hash: "".as_bytes().to_vec(),
        task_is_extra: Some(true),
        agent_id: Some(Addr::unchecked(AGENT0)),
        slot_kind: SlotType::Block,
    };

    //Notify agent got 1 task
    balancer.on_task_completed(
        &mut deps.storage,
        &env,
        &store.config,
        &store.agent_active_queue,
        task_info,
    );

    //Verify agent0 gets extra
    let slot: (Option<u64>, Option<u64>) = (Some(7), Some(7));
    let result = balancer
        .get_agent_tasks(
            &deps.as_ref(),
            &env.clone(),
            &store.config,
            &store.agent_active_queue,
            Addr::unchecked(AGENT0),
            slot,
        )
        .unwrap()
        .unwrap();

    //In equalizer mode, agent0 get 1 task and 0 extra
    assert_eq!(result.num_block_tasks.u64(), 1);
    assert_eq!(result.num_cron_tasks.u64(), 1);
    assert_eq!(result.num_block_tasks_extra.u64(), 0);
    assert_eq!(result.num_cron_tasks_extra.u64(), 0);

    //Verify agent1 gets extra
    let result = balancer
        .get_agent_tasks(
            &deps.as_ref(),
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

    //Verify agent2 gets extra
    let result = balancer
        .get_agent_tasks(
            &deps.as_ref(),
            &env.clone(),
            &store.config,
            &store.agent_active_queue,
            Addr::unchecked(AGENT2),
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
            &deps.as_ref(),
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