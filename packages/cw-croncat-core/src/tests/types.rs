use crate::{
    error::CoreError,
    msg::TaskRequest,
    types::{Action, Boundary, BoundaryValidated, GenericBalance, Interval, Task},
};
use cosmwasm_std::{
    coins, testing::mock_dependencies, Addr, BankMsg, Binary, Coin, CosmosMsg, GovMsg, IbcMsg,
    IbcTimeout, StdError, Timestamp, Uint64, VoteOption, WasmMsg,
};
use cw20::Cw20CoinVerified;
use cw_rules_core::types::{HasBalanceGte, Rule};
use hex::ToHex;
use sha2::{Digest, Sha256};

#[test]
fn is_valid_msg_once_block_based() {
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(4u64)),
            end: Some(Uint64::from(8u64)),
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "alice".to_string(),
                msg: Binary::from(vec![]),
                funds: vec![Coin::new(10, "coin")],
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert!(task
        .is_valid_msg_calculate_usage(
            &mock_dependencies().api,
            &Addr::unchecked("alice2"),
            &Addr::unchecked("bob"),
            &Addr::unchecked("bob"),
            5,
            5
        )
        .is_ok());
}

#[test]
fn is_valid_msg_once_time_based() {
    let task = TaskRequest {
        interval: Interval::Once,
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(1_000_000_000_u64)),
            end: Some(Uint64::from(2_000_000_000_u64)),
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "alice".to_string(),
                msg: Binary::from(vec![]),
                funds: vec![Coin::new(10, "coin")],
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert!(task
        .is_valid_msg_calculate_usage(
            &mock_dependencies().api,
            &Addr::unchecked("alice2"),
            &Addr::unchecked("bob"),
            &Addr::unchecked("bob"),
            5,
            5,
        )
        .is_ok());
}

#[test]
fn is_valid_msg_recurring() {
    let task = TaskRequest {
        interval: Interval::Block(10),
        boundary: None,
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "alice".to_string(),
                msg: Binary::from(vec![]),
                funds: vec![Coin::new(10, "coin")],
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert!(task
        .is_valid_msg_calculate_usage(
            &mock_dependencies().api,
            &Addr::unchecked("alice2"),
            &Addr::unchecked("bob"),
            &Addr::unchecked("bob"),
            5,
            5,
        )
        .is_ok());
}

#[test]
fn is_valid_msg_wrong_account() {
    // Cannot create a task to execute on the cron manager when not the owner
    let task = TaskRequest {
        interval: Interval::Block(5),
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(4u64)),
            end: None,
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "alice".to_string(),
                msg: Binary::from(vec![]),
                funds: vec![Coin::new(10, "coin")],
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert_eq!(
        CoreError::InvalidAction {},
        task.is_valid_msg_calculate_usage(
            &mock_dependencies().api,
            &Addr::unchecked("alice"),
            &Addr::unchecked("sender"),
            &Addr::unchecked("bob"),
            5,
            5
        )
        .unwrap_err()
    );
}

#[test]
fn is_valid_msg_vote() {
    // A task with CosmosMsg::Gov Vote should return false
    let task = TaskRequest {
        interval: Interval::Block(5),
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(4u64)),
            end: None,
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Gov(GovMsg::Vote {
                proposal_id: 0,
                vote: VoteOption::Yes,
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert_eq!(
        CoreError::InvalidAction {},
        task.is_valid_msg_calculate_usage(
            &mock_dependencies().api,
            &Addr::unchecked("alice"),
            &Addr::unchecked("sender"),
            &Addr::unchecked("bob"),
            5,
            5
        )
        .unwrap_err()
    );
}

#[test]
fn is_valid_msg_transfer() {
    // A task with CosmosMsg::Ibc Transfer should return false
    let task = TaskRequest {
        interval: Interval::Block(5),
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(4u64)),
            end: None,
        }),
        stop_on_fail: false,
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
        cw20_coins: Default::default(),
    };
    assert_eq!(
        CoreError::InvalidAction {},
        task.is_valid_msg_calculate_usage(
            &mock_dependencies().api,
            &Addr::unchecked("alice"),
            &Addr::unchecked("sender"),
            &Addr::unchecked("bob"),
            5,
            5
        )
        .unwrap_err()
    );
}

#[test]
fn is_valid_msg_burn() {
    // A task with CosmosMsg::Bank Burn should return false
    let task = TaskRequest {
        interval: Interval::Block(5),
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(4u64)),
            end: None,
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Bank(BankMsg::Burn {
                amount: vec![Coin::new(10, "coin")],
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert_eq!(
        CoreError::InvalidAction {},
        task.is_valid_msg_calculate_usage(
            mock_dependencies().as_ref().api,
            &Addr::unchecked("alice"),
            &Addr::unchecked("sender"),
            &Addr::unchecked("bob"),
            5,
            5
        )
        .unwrap_err()
    );
}

#[test]
fn is_valid_msg_send_doesnt_fail() {
    // A task with CosmosMsg::Bank Send should return true
    let task = TaskRequest {
        interval: Interval::Block(5),
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(4u64)),
            end: None,
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "address".to_string(),
                amount: vec![Coin::new(10, "coin")],
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert!(task
        .is_valid_msg_calculate_usage(
            mock_dependencies().as_ref().api,
            &Addr::unchecked("alice"),
            &Addr::unchecked("sender"),
            &Addr::unchecked("bob"),
            5,
            5
        )
        .is_ok());
}

#[test]
fn is_valid_msg_send_should_success() {
    // A task with CosmosMsg::Bank Send should return false
    let task = TaskRequest {
        interval: Interval::Block(1),
        boundary: Some(Boundary::Height {
            start: Some(Uint64::from(4u64)),
            end: None,
        }),
        stop_on_fail: false,
        actions: vec![Action {
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "address".to_string(),
                amount: vec![Coin::new(10, "atom")],
            }),
            gas_limit: Some(5),
        }],
        rules: None,
        cw20_coins: Default::default(),
    };
    assert!(task
        .is_valid_msg_calculate_usage(
            mock_dependencies().as_ref().api,
            &Addr::unchecked("alice"),
            &Addr::unchecked("sender"),
            &Addr::unchecked("bob"),
            5,
            5
        )
        .is_ok());
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
        funds_withdrawn_recurring: vec![],
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
