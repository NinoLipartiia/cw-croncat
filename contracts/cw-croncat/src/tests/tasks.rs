use super::helpers::{ADMIN, ANYONE, NATIVE_DENOM, VERY_RICH};
use crate::contract::{GAS_ACTION_FEE_JUNO, GAS_BASE_FEE_JUNO, GAS_DENOMINATOR_DEFAULT_JUNO};
use crate::tests::helpers::proper_instantiate;
use crate::ContractError;
use cosmwasm_std::{
    coin, coins, to_binary, Addr, BankMsg, CosmosMsg, StakingMsg, StdResult, Uint128, WasmMsg,
};
use cw_croncat_core::error::CoreError;
use cw_croncat_core::msg::{
    ExecuteMsg, GetBalancesResponse, GetSlotHashesResponse, GetSlotIdsResponse, QueryMsg,
    TaskRequest, TaskResponse, TaskWithRulesResponse,
};
use cw_croncat_core::types::{Action, Boundary, BoundaryValidated, GenericBalance, Interval, Task};
use cw_multi_test::Executor;
use cw_rules_core::types::{HasBalanceGte, Rule};
use std::convert::TryInto;

#[test]
fn query_task_hash_success() {
    let (app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let to_address = String::from("you");
    let amount = coins(1015, "earth");
    let bank = BankMsg::Send { to_address, amount };
    let msg: CosmosMsg = bank.clone().into();

    let task = Task {
        funds_withdrawn_recurring: vec![],
        owner_id: Addr::unchecked("nobody".to_string()),
        interval: Interval::Immediate,
        boundary: BoundaryValidated {
            start: None,
            end: None,
        },
        stop_on_fail: false,
        total_deposit: GenericBalance {
            native: coins(37, NATIVE_DENOM),
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
    let (app, cw_template_contract, _) = proper_instantiate();
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
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
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
        &coins(315006, NATIVE_DENOM),
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
    let (mut app, cw_template_contract, _) = proper_instantiate();
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
                    amount: coin(amount, NATIVE_DENOM),
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
            &coins(315000 + 2 * amount, NATIVE_DENOM),
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
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
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
        gas_fraction: None,
        proxy_callback_gas: None,
        slot_granularity_time: None,
        min_tasks_per_agent: None,
        gas_base_fee: None,
        gas_action_fee: None,
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
            &coins(315006, NATIVE_DENOM),
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
            gas_fraction: None,
            proxy_callback_gas: None,
            slot_granularity_time: None,
            min_tasks_per_agent: None,
            gas_base_fee: None,
            gas_action_fee: None,
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
    let res_err = app.execute_contract(
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
        &coins(13, NATIVE_DENOM),
    );
    assert_eq!(
        ContractError::CoreError(CoreError::InvalidAction {}),
        res_err.unwrap_err().downcast().unwrap()
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
            &coins(13, NATIVE_DENOM),
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
        &coins(315006, NATIVE_DENOM),
    )
    .unwrap();
    let res_err = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(315006, NATIVE_DENOM),
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
            &coins(315006, NATIVE_DENOM),
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
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
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
            &coins(315006, NATIVE_DENOM),
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
        assert_eq!(coins(315006, NATIVE_DENOM), t.total_deposit);
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
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
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
            &coins(315006, NATIVE_DENOM),
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
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
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
            &coins(315006, NATIVE_DENOM),
        )
        .unwrap();

    let res2 = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &without_rules_msg,
            &coins(315006, NATIVE_DENOM),
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
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
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
        &coins(315006, NATIVE_DENOM),
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
    assert_eq!(balances.available_balance.native, coins(1, NATIVE_DENOM));

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
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
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
        &coins(315006, NATIVE_DENOM),
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
            &coins(3, NATIVE_DENOM),
        )
        .unwrap();
    // Assert returned event attributes include total
    let mut matches_new_totals: bool = false;
    for e in res.events {
        for a in e.attributes {
            if a.key == "total_deposit" && a.value == r#"["315009atom"]"#.to_string() {
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
        assert_eq!(coins(315009, NATIVE_DENOM), t.total_deposit);
    }

    // Check the balance has increased to include the new refilled total
    let balances: GetBalancesResponse = app
        .wrap()
        .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetBalances {})
        .unwrap();
    assert_eq!(
        coins(315010, NATIVE_DENOM),
        balances.available_balance.native
    );

    Ok(())
}

#[test]
fn check_gas_minimum() {
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
    let stake = StakingMsg::Delegate { validator, amount };
    let msg: CosmosMsg = stake.clone().into();
    let gas_limit = 150_000;
    let base_gas = GAS_BASE_FEE_JUNO;

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
    let gas_for_two = (base_gas + gas_limit) * 2;
    let enough_for_two =
        u128::from((gas_for_two + gas_for_two * 5 / 100) / GAS_DENOMINATOR_DEFAULT_JUNO + 3 * 2);
    let res: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(enough_for_two - 1, NATIVE_DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        res,
        ContractError::CoreError(CoreError::NotEnoughNative {
            denom: NATIVE_DENOM.to_string(),
            lack: Uint128::from(1u128)
        })
    );

    // create a task
    let res = app.execute_contract(
        Addr::unchecked(ANYONE),
        contract_addr.clone(),
        &create_task_msg,
        &coins(enough_for_two, NATIVE_DENOM),
    );
    assert!(res.is_ok());
}

#[test]
fn check_gas_default() {
    let (mut app, cw_template_contract, _) = proper_instantiate();
    let contract_addr = cw_template_contract.addr();

    let validator = String::from("you");
    let amount = coin(3, NATIVE_DENOM);
    let stake = StakingMsg::Delegate { validator, amount };
    let msg: CosmosMsg = stake.clone().into();
    let gas_limit = GAS_ACTION_FEE_JUNO;
    let base_gas = GAS_BASE_FEE_JUNO;
    // let send = BankMsg::Send {
    //     to_address: validator,
    //     amount: vec![amount],
    // };

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

    let gas_for_two = (base_gas + gas_limit) * 2;
    let enough_for_two =
        u128::from((gas_for_two + gas_for_two * 5 / 100) / GAS_DENOMINATOR_DEFAULT_JUNO + 3 * 2);
    let res: ContractError = app
        .execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(enough_for_two - 1, NATIVE_DENOM),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        res,
        ContractError::CoreError(CoreError::NotEnoughNative {
            denom: NATIVE_DENOM.to_string(),
            lack: Uint128::from(1u128)
        })
    );

    // create a task
    let res = app.execute_contract(
        Addr::unchecked(ANYONE),
        contract_addr.clone(),
        &create_task_msg,
        &coins(enough_for_two, NATIVE_DENOM),
    );
    assert!(res.is_ok());
}
