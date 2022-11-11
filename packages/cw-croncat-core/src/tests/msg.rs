use cosmwasm_std::{coin, coins, Addr, BankMsg, CosmosMsg, Timestamp, Uint64};
use cw20::Cw20CoinVerified;

use crate::{
    msg::{
        AgentResponse, AgentTaskResponse, Croncat, GetAgentIdsResponse, GetBalancesResponse,
        GetConfigResponse, GetSlotHashesResponse, GetSlotIdsResponse, GetWalletBalancesResponse,
        TaskRequest, TaskResponse,
    },
    types::{
        Action, Agent, AgentStatus, Boundary, BoundaryValidated, GasFraction, GenericBalance,
        Interval, SlotType, Task,
    },
};

#[test]
fn everything_can_be_de_serialized() {
    let generic_balance = GenericBalance {
        native: vec![coin(5, "test")],
        cw20: vec![Cw20CoinVerified {
            address: Addr::unchecked("juno1"),
            amount: 125u128.into(),
        }],
    };
    let agent = Agent {
        payable_account_id: Addr::unchecked("test"),
        balance: generic_balance.clone(),
        total_tasks_executed: 0,
        last_executed_slot: 3,
        register_start: Timestamp::from_nanos(5),
    }
    .into();

    let msg: CosmosMsg = BankMsg::Send {
        to_address: "you".to_string(),
        amount: coins(1015, "earth"),
    }
    .into();

    let task = Task {
        funds_withdrawn_recurring: vec![],
        owner_id: Addr::unchecked("nobody".to_string()),
        interval: Interval::Immediate,
        boundary: BoundaryValidated {
            start: Some(54),
            end: Some(44),
        },
        stop_on_fail: false,
        total_deposit: Default::default(),
        amount_for_one_task: Default::default(),
        actions: vec![Action {
            msg,
            gas_limit: Some(150_000),
        }],
        rules: None,
    }
    .into();

    let config_response = GetConfigResponse {
        paused: true,
        owner_id: Addr::unchecked("bob"),
        min_tasks_per_agent: 5,
        agent_active_indices: vec![(SlotType::Block, 10, 5)],
        agents_eject_threshold: 5,
        agent_fee: 5,
        gas_fraction: GasFraction {
            numerator: 1,
            denominator: 2,
        },
        proxy_callback_gas: 3,
        slot_granularity_time: 60_000_000,
        native_denom: "juno".to_string(),
        cw_rules_addr: Addr::unchecked("bob"),
        agent_nomination_duration: 10,
        gas_base_fee: 1,
        gas_action_fee: 2,
        cw20_whitelist: vec![],
        available_balance: GenericBalance::default(),
        staked_balance: GenericBalance::default(),
        limit: 100,
    }
    .into();
    let balance_response = GetBalancesResponse {
        native_denom: "some".to_string(),
        available_balance: generic_balance.clone(),
        staked_balance: generic_balance.clone(),
        cw20_whitelist: vec![Addr::unchecked("bob")],
    }
    .into();
    let get_agent_ids_response = GetAgentIdsResponse {
        active: vec![Addr::unchecked("bob")],
        pending: vec![Addr::unchecked("bob")],
    }
    .into();
    let get_agent_tasks_response = AgentTaskResponse {
        num_block_tasks: 1u64.into(),
        num_block_tasks_extra: 2u64.into(),
        num_cron_tasks: 3u64.into(),
        num_cron_tasks_extra: 300u64.into(),
    }
    .into();
    let task_request = TaskRequest {
        interval: Interval::Block(5),
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(5u64)),
            end: Some(Uint64::from(64u64)),
        }),
        stop_on_fail: true,
        actions: vec![],
        rules: None, // TODO
        cw20_coins: vec![],
    }
    .into();
    let task_response_raw = TaskResponse {
        task_hash: "test".to_string(),
        owner_id: Addr::unchecked("bob"),
        interval: Interval::Cron("blah-blah".to_string()),
        boundary: Some(Boundary::Time {
            start: Some(Timestamp::from_nanos(12345)),
            end: Some(Timestamp::from_nanos(67890)),
        }),
        stop_on_fail: true,
        total_deposit: vec![coin(5, "earth")],
        total_cw20_deposit: vec![],
        amount_for_one_task_native: vec![coin(4, "earth")],
        amount_for_one_task_cw20: vec![],
        actions: vec![],
        rules: None,
        funds_withdrawn_recurring: vec![],
    };
    let task_response = task_response_raw.clone().into();
    let validate_interval_response = false.into();
    let get_agent_response = Some(AgentResponse {
        status: AgentStatus::Active,
        payable_account_id: Addr::unchecked("bob"),
        balance: generic_balance.clone(),
        total_tasks_executed: 2,
        last_executed_slot: 2,
        register_start: Timestamp::from_nanos(5),
    })
    .into();
    let get_tasks_response = vec![task_response_raw.clone()].into();
    let get_tasks_by_owner_response = vec![task_response_raw.clone()].into();
    let get_task_response = Some(task_response_raw).into();
    let get_task_hash_response = ("asd".to_string()).into();
    let get_slot_hashes_response = GetSlotHashesResponse {
        block_id: 5,
        block_task_hash: vec!["bob".to_string()],
        time_id: 4,
        time_task_hash: vec!["alice".to_string()],
    }
    .into();
    let get_slot_ids_response = GetSlotIdsResponse {
        time_ids: vec![1],
        block_ids: vec![3],
    }
    .into();
    let get_wallet_balances_response = GetWalletBalancesResponse {
        cw20_balances: vec![Cw20CoinVerified {
            address: Addr::unchecked("Bob"),
            amount: 5u128.into(),
        }],
    }
    .into();
    let croncat = Croncat {
        agent,
        task,
        config_response,
        balance_response,
        get_agent_ids_response,
        get_agent_tasks_response,
        task_request,
        task_response,
        validate_interval_response,
        get_agent_response,
        get_tasks_response,
        get_tasks_by_owner_response,
        get_task_response,
        get_task_hash_response,
        get_slot_hashes_response,
        get_slot_ids_response,
        get_wallet_balances_response,
    };

    let ser = serde_json_wasm::to_string(&croncat);
    assert!(ser.is_ok());

    let deser: Result<Croncat, _> = serde_json_wasm::from_str(&ser.unwrap());
    assert!(deser.is_ok());
}
