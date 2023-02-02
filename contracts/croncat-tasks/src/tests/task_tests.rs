use cosmwasm_std::{
    coin, coins, to_binary, Addr, BankMsg, Binary, StakingMsg, StdError, Uint128, Uint64, WasmMsg,
};
use croncat_sdk_core::types::AmountForOneTask;
use croncat_sdk_manager::types::TaskBalance;
use croncat_sdk_tasks::{
    msg::UpdateConfigMsg,
    types::{
        Action, Boundary, Config, CroncatQuery, Interval, SlotTasksTotalResponse, TaskInfo,
        TaskRequest, TaskResponse, Transform,
    },
};
use cw20::Cw20ExecuteMsg;
use cw_multi_test::{BankSudo, Executor};
use cw_storage_plus::KeyDeserialize;

use super::{
    contracts,
    helpers::{
        default_app, default_instantiate_msg, init_agents, init_factory, init_manager, init_tasks,
    },
    ADMIN, DENOM,
};
use crate::{
    contract::{GAS_ACTION_FEE, GAS_BASE_FEE, GAS_LIMIT, GAS_QUERY_FEE, SLOT_GRANULARITY_TIME},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{BLOCK_MAP_QUERIES, TASKS_TOTAL, TASKS_WITH_QUERIES_TOTAL, TIME_MAP_QUERIES},
    tests::{helpers::add_little_time, ANYONE},
    ContractError,
};

mod instantiate_tests {
    use super::*;

    #[test]
    fn default_init() {
        let mut app = default_app();
        let factory_addr = init_factory(&mut app);

        let instantiate_msg: InstantiateMsg = default_instantiate_msg();
        let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
        let config: Config = app
            .wrap()
            .query_wasm_smart(tasks_addr, &QueryMsg::Config {})
            .unwrap();
        let expected_config = Config {
            paused: false,
            version: "0.1".to_owned(),
            owner_addr: factory_addr.clone(),
            croncat_factory_addr: factory_addr,
            chain_name: "atom".to_owned(),
            croncat_manager_key: ("manager".to_owned(), [0, 1]),
            croncat_agents_key: ("agents".to_owned(), [0, 1]),
            slot_granularity_time: SLOT_GRANULARITY_TIME,
            gas_base_fee: GAS_BASE_FEE,
            gas_action_fee: GAS_ACTION_FEE,
            gas_query_fee: GAS_QUERY_FEE,
            gas_limit: GAS_LIMIT,
        };

        assert_eq!(config, expected_config);
    }

    #[test]
    fn custom_init() {
        let mut app = default_app();
        let factory_addr = init_factory(&mut app);

        let instantiate_msg: InstantiateMsg = InstantiateMsg {
            chain_name: "cron".to_owned(),
            version: Some("0.1".to_owned()),
            owner_addr: Some(ANYONE.to_owned()),
            croncat_manager_key: ("definitely_not_manager".to_owned(), [4, 2]),
            croncat_agents_key: ("definitely_not_agents".to_owned(), [42, 0]),
            slot_granularity_time: Some(10),
            gas_base_fee: Some(1),
            gas_action_fee: Some(2),
            gas_query_fee: Some(3),
            gas_limit: Some(10),
        };
        let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
        let config: Config = app
            .wrap()
            .query_wasm_smart(tasks_addr, &QueryMsg::Config {})
            .unwrap();

        let expected_config = Config {
            paused: false,
            version: "0.1".to_owned(),
            owner_addr: Addr::unchecked(ANYONE),
            croncat_factory_addr: factory_addr,
            chain_name: "cron".to_owned(),
            croncat_manager_key: ("definitely_not_manager".to_owned(), [4, 2]),
            croncat_agents_key: ("definitely_not_agents".to_owned(), [42, 0]),
            slot_granularity_time: 10,
            gas_base_fee: 1,
            gas_action_fee: 2,
            gas_query_fee: 3,
            gas_limit: 10,
        };
        assert_eq!(config, expected_config);
    }

    #[test]
    fn failed_inits() {
        let mut app = default_app();
        let code_id = app.store_code(contracts::croncat_tasks_contract());

        let instantiate_msg: InstantiateMsg = InstantiateMsg {
            owner_addr: Some("InVA$LID_ADDR".to_owned()),
            ..default_instantiate_msg()
        };
        let contract_err: ContractError = app
            .instantiate_contract(
                code_id,
                Addr::unchecked(ADMIN),
                &instantiate_msg,
                &[],
                "tasks",
                None,
            )
            .unwrap_err()
            .downcast()
            .unwrap();

        assert_eq!(
            contract_err,
            ContractError::Std(StdError::generic_err(
                "Invalid input: address not normalized"
            ))
        );
    }
}

#[test]
fn create_task_without_query() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
    let manager_addr = init_manager(&mut app, &factory_addr);
    let _ = init_agents(&mut app, &factory_addr);

    let action1 = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(50_000),
    };

    let action2 = Action {
        msg: BankMsg::Send {
            to_address: "Alice".to_owned(),
            amount: coins(10, DENOM),
        }
        .into(),
        gas_limit: Some(100_000),
    };

    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some((app.block_info().height).into()),
            end: Some((app.block_info().height + 10).into()),
        }),
        stop_on_fail: false,
        actions: vec![action1.clone(), action2.clone()],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap();

    // check it created task with responded task hash and can be queried from anywhere
    let task_hash = String::from_vec(res.data.unwrap().0).unwrap();
    assert!(task_hash.starts_with("atom:"));
    let tasks: Vec<TaskInfo> = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::Tasks {
                from_index: None,
                limit: None,
            },
        )
        .unwrap();
    let task_response: TaskResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::Task {
                task_hash: task_hash.clone(),
            },
        )
        .unwrap();
    assert_eq!(task_response.task.clone().unwrap(), tasks[0]);
    let expected_block_task_response = TaskResponse {
        task: Some(TaskInfo {
            task_hash: task_hash.clone(),
            owner_addr: Addr::unchecked(ANYONE),
            interval: Interval::Once,
            boundary: Boundary::Height {
                start: Some(app.block_info().height.into()),
                end: Some((app.block_info().height + 10).into()),
            },
            stop_on_fail: false,
            amount_for_one_task: AmountForOneTask {
                gas: GAS_BASE_FEE + action1.gas_limit.unwrap() + action2.gas_limit.unwrap(),
                cw20: None,
                coin: [Some(coin(15, DENOM)), None],
            },
            actions: vec![action1, action2],
            queries: None,
            transforms: vec![],
            version: "0.1".to_owned(),
        }),
    };
    assert_eq!(task_response.task, expected_block_task_response.task);

    // check total tasks
    let total_tasks: Uint64 = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::TasksTotal {})
        .unwrap();
    assert_eq!(total_tasks, Uint64::new(1));

    // check it created balance on the manager contract
    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr.clone(),
            &croncat_manager::msg::QueryMsg::TaskBalance { task_hash },
        )
        .unwrap();
    assert_eq!(
        manager_task_balance,
        Some(TaskBalance {
            native_balance: Uint128::new(30000),
            cw20_balance: None,
            ibc_balance: None,
        }),
    );

    // Check it's next item
    let current_slot: TaskResponse = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::CurrentTask {})
        .unwrap();
    assert!(current_slot.task.is_none());
    app.update_block(add_little_time);
    let slot_total: SlotTasksTotalResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::SlotTasksTotal { offset: None },
        )
        .unwrap();
    assert_eq!(
        slot_total,
        SlotTasksTotalResponse {
            block_tasks: 1,
            cron_tasks: 0,
        }
    );
    let current_slot: TaskResponse = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::CurrentTask {})
        .unwrap();

    assert_eq!(current_slot.task, expected_block_task_response.task);

    // check it all transferred out of tasks
    let manager_balance = app
        .wrap()
        .query_balance(manager_addr.clone(), DENOM)
        .unwrap();
    let tasks_balance = app.wrap().query_balance(tasks_addr.clone(), DENOM).unwrap();
    assert_eq!(manager_balance, coin(30000, DENOM));
    assert_eq!(tasks_balance, coin(0, DENOM));

    // Create second task do same checks, but add second coin
    app.sudo(
        BankSudo::Mint {
            to_address: ANYONE.to_owned(),
            amount: coins(10, "test_coins"),
        }
        .into(),
    )
    .unwrap();
    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: vec![coin(10, DENOM), coin(5, "test_coins")],
        }
        .into(),
        gas_limit: Some(60_000),
    };
    let task = TaskRequest {
        interval: Interval::Immediate,
        boundary: Some(Boundary::Time {
            start: Some(app.block_info().time),
            end: Some(app.block_info().time.plus_nanos(100)),
        }),
        stop_on_fail: false,
        actions: vec![action.clone()],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &[coin(60000, DENOM), coin(10, "test_coins")],
        )
        .unwrap();

    let task_hash = String::from_vec(res.data.unwrap().0).unwrap();
    assert!(task_hash.starts_with("atom:"));
    let responses: Vec<TaskInfo> = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::TasksByOwner {
                owner_addr: ANYONE.to_owned(),
                from_index: None,
                limit: None,
            },
        )
        .unwrap();
    let task_from_task_list = responses
        .into_iter()
        .find(|task_res| task_res.clone().task_hash == task_hash)
        .unwrap();
    let task_response: TaskResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::Task {
                task_hash: task_hash.clone(),
            },
        )
        .unwrap();
    assert_eq!(task_response.task.clone().unwrap(), task_from_task_list);

    let expected_cron_task_response = TaskResponse {
        task: Some(TaskInfo {
            task_hash: task_hash.clone(),
            owner_addr: Addr::unchecked(ANYONE),
            interval: Interval::Immediate,
            boundary: Boundary::Time {
                start: Some(app.block_info().time),
                end: Some(app.block_info().time.plus_nanos(100)),
            },
            stop_on_fail: false,
            amount_for_one_task: AmountForOneTask {
                gas: GAS_BASE_FEE + action.gas_limit.unwrap(),
                cw20: None,
                coin: [Some(coin(10, DENOM)), Some(coin(5, "test_coins"))],
            },
            actions: vec![action],
            queries: None,
            transforms: vec![],
            version: "0.1".to_owned(),
        }),
    };
    assert_eq!(task_response.task, expected_cron_task_response.task);

    let total_tasks: Uint64 = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::TasksTotal {})
        .unwrap();
    assert_eq!(total_tasks, Uint64::new(2));
    // Check that tasks doesn't overlap with tasks_with_queries
    let total_without_q = TASKS_TOTAL.query(&app.wrap(), tasks_addr.clone()).unwrap();
    assert_eq!(total_without_q, 2);
    let total_with_q = TASKS_WITH_QUERIES_TOTAL
        .query(&app.wrap(), tasks_addr.clone())
        .unwrap();
    assert_eq!(total_with_q, 0);

    // Check it got queued into correct slot
    app.update_block(add_little_time);
    let slot_total: SlotTasksTotalResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::SlotTasksTotal { offset: None },
        )
        .unwrap();
    assert_eq!(
        slot_total,
        SlotTasksTotalResponse {
            block_tasks: 1,
            cron_tasks: 1,
        }
    );

    // Check it prefers block over cron
    let current_slot: TaskResponse = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::CurrentTask {})
        .unwrap();
    assert_eq!(current_slot.task, expected_block_task_response.task);

    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr.clone(),
            &croncat_manager::msg::QueryMsg::TaskBalance { task_hash },
        )
        .unwrap();
    assert_eq!(
        manager_task_balance,
        Some(TaskBalance {
            native_balance: Uint128::new(60000),
            cw20_balance: None,
            ibc_balance: Some(coin(10, "test_coins")),
        }),
    );

    let manager_balance = app.wrap().query_all_balances(manager_addr).unwrap();
    let tasks_balance = app.wrap().query_all_balances(tasks_addr).unwrap();
    assert_eq!(
        manager_balance,
        vec![coin(30000 + 60000, DENOM), coin(10, "test_coins")]
    );
    assert_eq!(tasks_balance, vec![]);
}

#[test]
fn create_task_with_wasm() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
    let manager_addr = init_manager(&mut app, &factory_addr);
    let _ = init_agents(&mut app, &factory_addr);

    let action = Action {
        msg: WasmMsg::Execute {
            contract_addr: manager_addr.to_string(),
            msg: to_binary(&croncat_manager::msg::ExecuteMsg::Tick {}).unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: Some(150_000),
    };

    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some((app.block_info().height).into()),
            end: Some((app.block_info().height + 10).into()),
        }),
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(ADMIN),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap();
    let task_hash = String::from_vec(res.data.unwrap().0).unwrap();

    // check total tasks
    let total_tasks: Uint64 = app
        .wrap()
        .query_wasm_smart(tasks_addr, &QueryMsg::TasksTotal {})
        .unwrap();
    assert_eq!(total_tasks, Uint64::new(1));

    // check it created balance on the manager contract
    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr,
            &croncat_manager::msg::QueryMsg::TaskBalance { task_hash },
        )
        .unwrap();
    assert_eq!(
        manager_task_balance,
        Some(TaskBalance {
            native_balance: Uint128::new(30000),
            cw20_balance: None,
            ibc_balance: None,
        }),
    );
}

#[test]
fn create_tasks_with_queries_and_transforms() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
    let manager_addr = init_manager(&mut app, &factory_addr);
    let _ = init_agents(&mut app, &factory_addr);

    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(50_000),
    };
    let queries = vec![
        CroncatQuery {
            contract_addr: "aloha123".to_owned(),
            msg: Binary::from([4, 2]),
            check_result: true,
        },
        CroncatQuery {
            contract_addr: "aloha321".to_owned(),
            msg: Binary::from([2, 4]),
            check_result: true,
        },
    ];
    let transforms = vec![Transform {
        action_idx: 1,
        query_idx: 2,
        action_path: vec![5u64.into()].into(),
        query_response_path: vec![5u64.into()].into(),
    }];

    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some((app.block_info().height).into()),
            end: Some((app.block_info().height + 10).into()),
        }),
        stop_on_fail: false,
        actions: vec![action.clone()],
        queries: Some(queries.clone()),
        transforms: Some(transforms.clone()),
        cw20: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(50000, DENOM),
        )
        .unwrap();
    let task_hash = String::from_vec(res.data.unwrap().0).unwrap();
    let tasks: Vec<TaskInfo> = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::TasksWithQueries {
                from_index: None,
                limit: None,
            },
        )
        .unwrap();
    let task_response: TaskResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::Task {
                task_hash: task_hash.clone(),
            },
        )
        .unwrap();
    assert_eq!(task_response.task.clone().unwrap(), tasks[0]);

    let expected_block_task_response = TaskResponse {
        task: Some(TaskInfo {
            task_hash: task_hash.clone(),
            owner_addr: Addr::unchecked(ANYONE),
            interval: Interval::Once,
            boundary: Boundary::Height {
                start: Some(app.block_info().height.into()),
                end: Some((app.block_info().height + 10).into()),
            },
            stop_on_fail: false,
            amount_for_one_task: AmountForOneTask {
                gas: GAS_BASE_FEE + action.gas_limit.unwrap() + GAS_QUERY_FEE * 2,
                cw20: None,
                coin: [Some(coin(5, DENOM)), None],
            },
            actions: vec![action],
            queries: Some(queries),
            transforms,
            version: "0.1".to_owned(),
        }),
    };
    assert_eq!(task_response.task, expected_block_task_response.task);

    let total_tasks: Uint64 = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::TasksTotal {})
        .unwrap();
    assert_eq!(total_tasks, Uint64::new(1));
    let total_tasks_with_queries: Uint64 = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::TasksWithQueriesTotal {})
        .unwrap();
    assert_eq!(total_tasks_with_queries, Uint64::new(1));

    // Check that tasks doesn't overlap with tasks_with_queries
    let total_without_q = TASKS_TOTAL.query(&app.wrap(), tasks_addr.clone()).unwrap();
    assert_eq!(total_without_q, 0);
    let total_with_q = TASKS_WITH_QUERIES_TOTAL
        .query(&app.wrap(), tasks_addr.clone())
        .unwrap();
    assert_eq!(total_with_q, 1);

    // check it created balance on the manager contract
    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr.clone(),
            &croncat_manager::msg::QueryMsg::TaskBalance { task_hash },
        )
        .unwrap();
    assert_eq!(
        manager_task_balance,
        Some(TaskBalance {
            native_balance: Uint128::new(50000),
            cw20_balance: None,
            ibc_balance: None,
        }),
    );

    // should not be scheduled
    app.update_block(add_little_time);
    let slot_total: SlotTasksTotalResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::SlotTasksTotal { offset: None },
        )
        .unwrap();
    assert_eq!(
        slot_total,
        SlotTasksTotalResponse {
            block_tasks: 0,
            cron_tasks: 0,
        }
    );

    let manager_balance = app.wrap().query_all_balances(manager_addr).unwrap();
    let tasks_balance = app.wrap().query_all_balances(tasks_addr).unwrap();
    assert_eq!(manager_balance, vec![coin(50000, DENOM)]);
    assert_eq!(tasks_balance, vec![]);
}

#[test]
fn remove_tasks() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
    let manager_addr = init_manager(&mut app, &factory_addr);
    let _ = init_agents(&mut app, &factory_addr);

    // Create one block and one cron with queries and then remove one by one
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some((app.block_info().height).into()),
            end: Some((app.block_info().height + 10).into()),
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: BankMsg::Send {
                to_address: "Bob".to_owned(),
                amount: coins(5, DENOM),
            }
            .into(),
            gas_limit: Some(50_000),
        }],
        queries: Some(vec![
            CroncatQuery {
                contract_addr: "aloha123".to_owned(),
                msg: Binary::from([4, 2]),
                check_result: true,
            },
            CroncatQuery {
                contract_addr: "aloha321".to_owned(),
                msg: Binary::from([2, 4]),
                check_result: true,
            },
        ]),
        transforms: Some(vec![Transform {
            action_idx: 1,
            query_idx: 2,
            action_path: vec![5u64.into()].into(),
            query_response_path: vec![5u64.into()].into(),
        }]),
        cw20: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(50000, DENOM),
        )
        .unwrap();
    let task_hash_block_with_queries = String::from_vec(res.data.unwrap().0).unwrap();
    // Scheduled exactly for block
    let min_block_scheduled = BLOCK_MAP_QUERIES
        .query(
            &app.wrap(),
            tasks_addr.clone(),
            task_hash_block_with_queries.as_bytes(),
        )
        .unwrap();
    assert_eq!(min_block_scheduled, Some(app.block_info().height + 1));
    assert!(TIME_MAP_QUERIES
        .query(
            &app.wrap(),
            tasks_addr.clone(),
            task_hash_block_with_queries.as_bytes(),
        )
        .unwrap()
        .is_none());

    // check it created balance on the manager contract
    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr.clone(),
            &croncat_manager::msg::QueryMsg::TaskBalance {
                task_hash: task_hash_block_with_queries.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        manager_task_balance,
        Some(TaskBalance {
            native_balance: Uint128::new(50000),
            cw20_balance: None,
            ibc_balance: None,
        }),
    );

    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Time {
            start: Some(app.block_info().time),
            end: Some(app.block_info().time.plus_nanos(1000)),
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: BankMsg::Send {
                to_address: "Bob".to_owned(),
                amount: coins(5, DENOM),
            }
            .into(),
            gas_limit: Some(50_000),
        }],
        queries: Some(vec![
            CroncatQuery {
                contract_addr: "aloha123".to_owned(),
                msg: Binary::from([4, 2]),
                check_result: true,
            },
            CroncatQuery {
                contract_addr: "aloha321".to_owned(),
                msg: Binary::from([2, 4]),
                check_result: true,
            },
        ]),
        transforms: Some(vec![Transform {
            action_idx: 1,
            query_idx: 2,
            action_path: vec![5u64.into()].into(),
            query_response_path: vec![5u64.into()].into(),
        }]),
        cw20: None,
    };

    let res = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(50000, DENOM),
        )
        .unwrap();
    let task_hash_cron_with_queries = String::from_vec(res.data.unwrap().0).unwrap();

    // Scheduled exactly for cron
    let min_cron_scheduled = TIME_MAP_QUERIES
        .query(
            &app.wrap(),
            tasks_addr.clone(),
            task_hash_cron_with_queries.as_bytes(),
        )
        .unwrap();
    assert!(min_cron_scheduled.is_some());
    assert!(BLOCK_MAP_QUERIES
        .query(
            &app.wrap(),
            tasks_addr.clone(),
            task_hash_cron_with_queries.as_bytes(),
        )
        .unwrap()
        .is_none());

    // check it created balance on the manager contract
    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr.clone(),
            &croncat_manager::msg::QueryMsg::TaskBalance {
                task_hash: task_hash_cron_with_queries.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        manager_task_balance,
        Some(TaskBalance {
            native_balance: Uint128::new(50000),
            cw20_balance: None,
            ibc_balance: None,
        }),
    );

    // remove block task
    app.execute_contract(
        Addr::unchecked(ANYONE),
        tasks_addr.clone(),
        &ExecuteMsg::RemoveTask {
            task_hash: task_hash_block_with_queries.clone(),
        },
        &[],
    )
    .unwrap();

    let task_response: TaskResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::Task {
                task_hash: task_hash_block_with_queries.clone(),
            },
        )
        .unwrap();
    assert!(task_response.task.is_none());

    assert!(BLOCK_MAP_QUERIES
        .query(
            &app.wrap(),
            tasks_addr.clone(),
            task_hash_block_with_queries.as_bytes(),
        )
        .unwrap()
        .is_none());
    // check it removed balance on the manager contract
    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr.clone(),
            &croncat_manager::msg::QueryMsg::TaskBalance {
                task_hash: task_hash_block_with_queries,
            },
        )
        .unwrap();
    assert!(manager_task_balance.is_none());

    // remove cron task
    app.execute_contract(
        Addr::unchecked(ANYONE),
        tasks_addr.clone(),
        &ExecuteMsg::RemoveTask {
            task_hash: task_hash_cron_with_queries.clone(),
        },
        &[],
    )
    .unwrap();

    let task_response: TaskResponse = app
        .wrap()
        .query_wasm_smart(
            tasks_addr.clone(),
            &QueryMsg::Task {
                task_hash: task_hash_cron_with_queries.clone(),
            },
        )
        .unwrap();
    assert!(task_response.task.is_none());

    assert!(TIME_MAP_QUERIES
        .query(
            &app.wrap(),
            tasks_addr,
            task_hash_cron_with_queries.as_bytes(),
        )
        .unwrap()
        .is_none());
    // check it removed balance on the manager contract
    let manager_task_balance: Option<TaskBalance> = app
        .wrap()
        .query_wasm_smart(
            manager_addr.clone(),
            &croncat_manager::msg::QueryMsg::TaskBalance {
                task_hash: task_hash_cron_with_queries,
            },
        )
        .unwrap();
    assert!(manager_task_balance.is_none());

    // Check all balances moved from manager contract
    let manager_balance = app.wrap().query_all_balances(manager_addr).unwrap();
    assert!(manager_balance.is_empty());
}

#[test]
fn update_cfg() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);

    app.execute_contract(
        Addr::unchecked(ADMIN),
        tasks_addr.clone(),
        &ExecuteMsg::UpdateConfig(UpdateConfigMsg {
            paused: Some(true),
            owner_addr: Some(ANYONE.to_owned()),
            croncat_factory_addr: Some("fixed_croncat_factory_addr".to_owned()),
            croncat_manager_key: Some(("manager2".to_owned(), [2, 2])),
            croncat_agents_key: Some(("agents2".to_owned(), [2, 2])),
            slot_granularity_time: Some(54),
            gas_base_fee: Some(1),
            gas_action_fee: Some(2),
            gas_query_fee: Some(3),
            gas_limit: Some(4),
        }),
        &[],
    )
    .unwrap();
    let config: Config = app
        .wrap()
        .query_wasm_smart(tasks_addr.clone(), &QueryMsg::Config {})
        .unwrap();
    let expected_config = Config {
        paused: true,
        version: "0.1".to_owned(),
        owner_addr: Addr::unchecked(ANYONE),
        croncat_factory_addr: Addr::unchecked("fixed_croncat_factory_addr"),
        chain_name: "atom".to_owned(),
        croncat_manager_key: ("manager2".to_owned(), [2, 2]),
        croncat_agents_key: ("agents2".to_owned(), [2, 2]),
        slot_granularity_time: 54,
        gas_base_fee: 1,
        gas_action_fee: 2,
        gas_query_fee: 3,
        gas_limit: 4,
    };

    assert_eq!(config, expected_config);

    // None's shouldn't impact any of the fields
    app.execute_contract(
        Addr::unchecked(ADMIN),
        tasks_addr.clone(),
        &ExecuteMsg::UpdateConfig(UpdateConfigMsg {
            paused: None,
            owner_addr: None,
            croncat_factory_addr: None,
            croncat_manager_key: None,
            croncat_agents_key: None,
            slot_granularity_time: None,
            gas_base_fee: None,
            gas_action_fee: None,
            gas_query_fee: None,
            gas_limit: None,
        }),
        &[],
    )
    .unwrap();

    let not_updated_config: Config = app
        .wrap()
        .query_wasm_smart(tasks_addr, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(not_updated_config, expected_config);
}

#[test]
fn negative_create_task() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
    let _manager_addr = init_manager(&mut app, &factory_addr);
    let _agent_addr = init_agents(&mut app, &factory_addr);

    // invalid interval
    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(50_000),
    };

    let task = TaskRequest {
        interval: Interval::Cron("aloha".to_string()),
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidInterval {});
    // invalid gas limit
    let action1 = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(GAS_LIMIT / 2),
    };
    let action2 = Action {
        msg: BankMsg::Send {
            to_address: "Alice".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(GAS_LIMIT / 2 + 1),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some((app.block_info().height).into()),
            end: Some((app.block_info().height + 10).into()),
        }),
        stop_on_fail: false,
        actions: vec![action1, action2],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidGas {});

    // Invalid boundary
    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(25_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some((app.block_info().height).into()),
            end: Some((app.block_info().height).into()),
        }),
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidBoundary {});

    // Same task - can't repeat tasks
    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(25_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    app.execute_contract(
        Addr::unchecked(ANYONE),
        tasks_addr.clone(),
        &ExecuteMsg::CreateTask {
            task: Box::new(task.clone()),
        },
        &coins(30000, DENOM),
    )
    .unwrap();
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::TaskExists {});

    // Same task, but with queries - can't repeat tasks
    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(25_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: Some(vec![CroncatQuery {
            contract_addr: "aloha".to_owned(),
            msg: Default::default(),
            check_result: true,
        }]),
        transforms: None,
        cw20: None,
    };
    app.execute_contract(
        Addr::unchecked(ANYONE),
        tasks_addr.clone(),
        &ExecuteMsg::CreateTask {
            task: Box::new(task.clone()),
        },
        &coins(40000, DENOM),
    )
    .unwrap();
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(40000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::TaskExists {});
    // contract paused

    app.execute_contract(
        Addr::unchecked(ADMIN),
        tasks_addr.clone(),
        &ExecuteMsg::UpdateConfig(UpdateConfigMsg {
            paused: Some(true),
            owner_addr: None,
            croncat_factory_addr: None,
            croncat_manager_key: None,
            croncat_agents_key: None,
            slot_granularity_time: None,
            gas_base_fee: None,
            gas_action_fee: None,
            gas_query_fee: None,
            gas_limit: None,
        }),
        &[],
    )
    .unwrap();

    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(25_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr,
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Paused {});
}

#[test]
fn remove_task_negative() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
    let _manager_addr = init_manager(&mut app, &factory_addr);
    let _agent_addr = init_agents(&mut app, &factory_addr);

    let action = Action {
        msg: BankMsg::Send {
            to_address: "Bob".to_owned(),
            amount: coins(5, DENOM),
        }
        .into(),
        gas_limit: Some(25_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action.clone()],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(40000, DENOM),
        )
        .unwrap();

    let task_hash = String::from_vec(res.data.unwrap().0).unwrap();

    // Not task owner tries to remove a task
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("wrong_person"),
            tasks_addr.clone(),
            &ExecuteMsg::RemoveTask { task_hash },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::Unauthorized {});

    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: Some(vec![CroncatQuery {
            contract_addr: "aloha".to_owned(),
            msg: Default::default(),
            check_result: true,
        }]),
        transforms: None,
        cw20: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(40000, DENOM),
        )
        .unwrap();

    let task_hash = String::from_vec(res.data.unwrap().0).unwrap();

    // Not a task owner tries to remove a task with queries
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("wrong_person"),
            tasks_addr.clone(),
            &ExecuteMsg::RemoveTask {
                task_hash: task_hash.clone(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::Unauthorized {});

    // No task
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked("wrong_person"),
            tasks_addr.clone(),
            &ExecuteMsg::RemoveTask {
                task_hash: "wrong_task_hash".to_owned(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::NoTaskFound {});

    // contract is paused
    app.execute_contract(
        Addr::unchecked(ADMIN),
        tasks_addr.clone(),
        &ExecuteMsg::UpdateConfig(UpdateConfigMsg {
            paused: Some(true),
            owner_addr: None,
            croncat_factory_addr: None,
            croncat_manager_key: None,
            croncat_agents_key: None,
            slot_granularity_time: None,
            gas_base_fee: None,
            gas_action_fee: None,
            gas_query_fee: None,
            gas_limit: None,
        }),
        &[],
    )
    .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr,
            &ExecuteMsg::RemoveTask { task_hash },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::Paused {});
}

#[test]
fn is_valid_msg_negative_tests() {
    let mut app = default_app();
    let factory_addr = init_factory(&mut app);

    let instantiate_msg: InstantiateMsg = default_instantiate_msg();
    let tasks_addr = init_tasks(&mut app, &instantiate_msg, &factory_addr);
    let _manager_addr = init_manager(&mut app, &factory_addr);
    let _agent_addr = init_agents(&mut app, &factory_addr);

    // no actions
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // no gas limit for wasm action
    let wasm_action = Action {
        msg: WasmMsg::Execute {
            contract_addr: "contract".to_owned(),
            msg: to_binary("wasm message").unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: None,
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![wasm_action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoGasLimit {});

    // Too many coins bank transfer
    let action = Action {
        msg: BankMsg::Send {
            to_address: "alice".to_owned(),
            amount: vec![coin(5, "coin1"), coin(2, "coin2"), coin(45, "coin3")],
        }
        .into(),
        gas_limit: None,
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // Zero coin bank transfer
    let action = Action {
        msg: BankMsg::Send {
            to_address: "alice".to_owned(),
            amount: vec![coin(0, "coin1")],
        }
        .into(),
        gas_limit: None,
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // not supported message
    let action = Action {
        msg: BankMsg::Burn {
            amount: vec![coin(10, "coin1")],
        }
        .into(),
        gas_limit: None,
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // not supported message
    let action = Action {
        msg: StakingMsg::Delegate {
            validator: "alice".to_owned(),
            amount: coin(10, "coin1"),
        }
        .into(),
        gas_limit: None,
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // too many coins transfer inside wasm action
    let action = Action {
        msg: WasmMsg::Execute {
            contract_addr: "bestcontract".to_owned(),
            msg: Default::default(),
            funds: vec![coin(5, "coin1"), coin(2, "coin2"), coin(45, "coin3")],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // zero coins transfer inside wasm action
    let action = Action {
        msg: WasmMsg::Execute {
            contract_addr: "bestcontract".to_owned(),
            msg: Default::default(),
            funds: vec![coin(0, "coin1")],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // Zero cw20 transfer
    let action = Action {
        msg: WasmMsg::Execute {
            contract_addr: "bestcontract".to_owned(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "bob".to_owned(),
                amount: Uint128::new(0),
            })
            .unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // Zero cw20 send
    let action = Action {
        msg: WasmMsg::Execute {
            contract_addr: "best_contract".to_owned(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "bob".to_owned(),
                msg: Default::default(),
                amount: Uint128::new(0),
            })
            .unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // Multiple cw20 send
    let action1 = Action {
        msg: WasmMsg::Execute {
            contract_addr: "best_contract".to_owned(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "bob".to_owned(),
                msg: Default::default(),
                amount: Uint128::new(45),
            })
            .unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let action2 = Action {
        msg: WasmMsg::Execute {
            contract_addr: "best_contract2".to_owned(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "bob".to_owned(),
                msg: Default::default(),
                amount: Uint128::new(45),
            })
            .unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action1, action2],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr.clone(),
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});

    // Multiple cw20 transfer
    let action1 = Action {
        msg: WasmMsg::Execute {
            contract_addr: "best_contract".to_owned(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "bob".to_owned(),
                amount: Uint128::new(45),
            })
            .unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let action2 = Action {
        msg: WasmMsg::Execute {
            contract_addr: "best_contract2".to_owned(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "bob".to_owned(),
                amount: Uint128::new(45),
            })
            .unwrap(),
            funds: vec![],
        }
        .into(),
        gas_limit: Some(150_000),
    };
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![action1, action2],
        queries: None,
        transforms: None,
        cw20: None,
    };
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            tasks_addr,
            &ExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            &coins(30000, DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidAction {});
}
