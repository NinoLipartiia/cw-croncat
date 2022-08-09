use cosmwasm_std::{DepsMut, MessageInfo, Response};
use cw20::{Cw20CoinVerified, Cw20ReceiveMsg};
use cw_croncat_core::traits::BalancesOperations;

use crate::{ContractError, CwCroncat};

impl<'a> CwCroncat<'a> {
    /// Add cw20 coin to user balance, that sent this coins
    pub fn receive_cw20(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        msg: Cw20ReceiveMsg,
    ) -> Result<Response, ContractError> {
        let sender = deps.api.addr_validate(&msg.sender)?;
        let coin_address = info.sender;

        // Updating user balance
        let new_balances = self.balances.update(
            deps.storage,
            &sender,
            |balances| -> Result<_, ContractError> {
                let mut balances = balances.unwrap_or_default();
                balances.checked_add_coins(&[Cw20CoinVerified {
                    address: coin_address.clone(),
                    amount: msg.amount,
                }])?;
                Ok(balances)
            },
        )?;

        // Updating contract balance
        self.config
            .update(deps.storage, |mut c| -> Result<_, ContractError> {
                c.available_balance.checked_add_cw20(&[Cw20CoinVerified {
                    address: coin_address,
                    amount: msg.amount,
                }])?;
                Ok(c)
            })?;

        let total_cw20_string: Vec<String> = new_balances.iter().map(ToString::to_string).collect();
        Ok(Response::new()
            .add_attribute("method", "receive_cw20")
            .add_attribute("total_cw20_balances", format!("{total_cw20_string:?}")))
    }
}

#[cfg(test)]
mod test {
    use crate::ContractError;
    use cosmwasm_std::{
        coin, coins, to_binary, Addr, BlockInfo, CosmosMsg, Empty, StdError, Uint128, WasmMsg,
    };
    use cw20::{BalanceResponse, Cw20Coin, Cw20CoinVerified};
    use cw_croncat_core::error::CoreError;
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    // use cw20::Balance;
    use crate::helpers::CwTemplateContract;
    use cw_croncat_core::msg::{
        ExecuteMsg, GetWalletBalancesResponse, InstantiateMsg, QueryMsg, TaskRequest, TaskResponse,
    };
    use cw_croncat_core::types::{Action, Interval};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::entry::execute,
            crate::entry::instantiate,
            crate::entry::query,
        )
        .with_reply(crate::entry::reply);
        Box::new(contract)
    }

    pub fn cw20_template() -> Box<dyn Contract<Empty>> {
        let cw20 = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(cw20)
    }

    const ADMIN: &str = "cosmos1sjllsnramtg3ewxqwwrwjxfgc4n4ef9u0tvx7u";
    const ANYONE: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const AGENT0: &str = "cosmos1a7uhnpqthunr2rzj0ww0hwurpn42wyun6c5puz";
    const AGENT1_BENEFICIARY: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const NATIVE_DENOM: &str = "atom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            let accounts: Vec<(u128, String)> = vec![
                (6_000_000, ADMIN.to_string()),
                (500_000, ANYONE.to_string()),
                (2_000_000, AGENT0.to_string()),
                (2_000_000, AGENT1_BENEFICIARY.to_string()),
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

    fn proper_instantiate() -> (App, CwTemplateContract, Addr) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());
        let owner_addr = Addr::unchecked(ADMIN);

        let msg = InstantiateMsg {
            denom: NATIVE_DENOM.to_string(),
            owner_id: Some(owner_addr.clone()),
            gas_base_fee: None,
            agent_nomination_duration: None,
        };
        let cw_template_contract_addr = app
            //Must send some available balance for rewards
            .instantiate_contract(
                cw_template_id,
                owner_addr.clone(),
                &msg,
                &coins(2_000_000, NATIVE_DENOM),
                "Manager",
                None,
            )
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        let cw20_id = app.store_code(cw20_template());
        let msg = cw20_base::msg::InstantiateMsg {
            name: "test".to_string(),
            symbol: "tset".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: ANYONE.to_string(),
                amount: 10u128.into(),
            }],
            mint: None,
            marketing: None,
        };
        let cw20_addr = app
            .instantiate_contract(cw20_id, owner_addr, &msg, &[], "Fungible-tokens", None)
            .unwrap();
        (app, cw_template_contract, cw20_addr)
    }

    pub fn add_little_time(block: &mut BlockInfo) {
        // block.time = block.time.plus_seconds(360);
        block.time = block.time.plus_seconds(19);
        block.height += 1;
    }

    #[test]
    fn test_cw20_action() {
        let (mut app, cw_template_contract, cw20_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        // fill balance of cw20 tokens of user
        let user = ANYONE;
        let refill_balance_msg = cw20::Cw20ExecuteMsg::Send {
            contract: contract_addr.to_string(),
            amount: 10u128.into(),
            msg: Default::default(),
        };
        app.execute_contract(
            Addr::unchecked(user),
            cw20_contract.clone(),
            &refill_balance_msg,
            &[],
        )
        .unwrap();

        // create a task sending cw20 to AGENT0
        let msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: cw20_contract.to_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: AGENT0.to_string(),
                amount: 10u128.into(),
            })
            .unwrap(),
            funds: vec![],
        }
        .into();
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
                cw20_coins: vec![Cw20Coin {
                    address: cw20_contract.to_string(),
                    amount: 10u128.into(),
                }],
            },
        };
        app.execute_contract(
            Addr::unchecked(user),
            contract_addr.clone(),
            &create_task_msg,
            &coins(u128::from(300_010_u128), "atom"),
        )
        .unwrap();

        // quick agent register
        {
            let msg = ExecuteMsg::RegisterAgent {
                payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
            };
            app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
                .unwrap();
        }

        app.update_block(add_little_time);

        // Agent executes transfer
        let proxy_call_msg = ExecuteMsg::ProxyCall {};
        app.execute_contract(
            Addr::unchecked(AGENT0),
            contract_addr.clone(),
            &proxy_call_msg,
            &vec![],
        )
        .unwrap();

        // Check new balance of AGENT0
        let balance: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_contract,
                &cw20::Cw20QueryMsg::Balance {
                    address: AGENT0.to_string(),
                },
            )
            .unwrap();
        assert_eq!(
            balance,
            BalanceResponse {
                balance: 10_u128.into()
            }
        );
    }

    #[test]
    fn test_cw20_balances() {
        let (mut app, cw_template_contract, cw20_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        // fill balance of cw20 tokens of user
        let user = ANYONE;
        // Balances before refill
        let balances: GetWalletBalancesResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetWalletBalances {
                    wallet: user.to_string(),
                },
            )
            .unwrap();
        assert!(balances.cw20_balances.is_empty());

        let refill_balance_msg = cw20::Cw20ExecuteMsg::Send {
            contract: contract_addr.to_string(),
            amount: 10u128.into(),
            msg: Default::default(),
        };
        app.execute_contract(
            Addr::unchecked(user),
            cw20_contract.clone(),
            &refill_balance_msg,
            &[],
        )
        .unwrap();

        // Check Balances of user after refill
        let balances: GetWalletBalancesResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetWalletBalances {
                    wallet: user.to_string(),
                },
            )
            .unwrap();
        assert_eq!(
            balances,
            GetWalletBalancesResponse {
                cw20_balances: vec![Cw20CoinVerified {
                    address: cw20_contract.clone(),
                    amount: 10u128.into()
                }]
            }
        );

        // create a task sending cw20 to AGENT0
        let msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: cw20_contract.to_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: AGENT0.to_string(),
                amount: 10u128.into(),
            })
            .unwrap(),
            funds: vec![],
        }
        .into();
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
                cw20_coins: vec![Cw20Coin {
                    address: cw20_contract.to_string(),
                    amount: 10u128.into(),
                }],
            },
        };
        let mut resp = app
            .execute_contract(
                Addr::unchecked(user),
                contract_addr.clone(),
                &create_task_msg,
                &coins(u128::from(300_010_u128), "atom"),
            )
            .unwrap();
        let task_hash = resp
            .events
            .pop()
            .unwrap()
            .attributes
            .into_iter()
            .find(|attr| attr.key == "task_hash")
            .unwrap();

        // Check task balances increased
        let task: TaskResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_hash.value,
                },
            )
            .unwrap();
        assert_eq!(
            task.total_cw20_deposit,
            vec![Cw20CoinVerified {
                address: cw20_contract.clone(),
                amount: 10u128.into()
            }]
        );
        // And user balances decreased
        let balances: GetWalletBalancesResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetWalletBalances {
                    wallet: user.to_string(),
                },
            )
            .unwrap();
        assert!(balances.cw20_balances.is_empty());
    }

    #[test]
    fn test_cw20_negative() {
        let (mut app, cw_template_contract, cw20_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let user = ANYONE;

        // create a task with empty balance
        let msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: cw20_contract.to_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: AGENT0.to_string(),
                amount: 10u128.into(),
            })
            .unwrap(),
            funds: vec![],
        }
        .into();
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
                cw20_coins: vec![Cw20Coin {
                    address: cw20_contract.to_string(),
                    amount: 10u128.into(),
                }],
            },
        };
        let resp: ContractError = app
            .execute_contract(
                Addr::unchecked(user),
                contract_addr.clone(),
                &create_task_msg,
                &coins(u128::from(300_010_u128), "atom"),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(resp, ContractError::CoreError(CoreError::EmptyBalance {}));
        // or with not enough balance

        // fill balance of cw20 tokens of user
        let refill_balance_msg = cw20::Cw20ExecuteMsg::Send {
            contract: contract_addr.to_string(),
            amount: 9u128.into(),
            msg: Default::default(),
        };
        app.execute_contract(
            Addr::unchecked(user),
            cw20_contract.clone(),
            &refill_balance_msg,
            &[],
        )
        .unwrap();

        let resp: ContractError = app
            .execute_contract(
                Addr::unchecked(user),
                contract_addr.clone(),
                &create_task_msg,
                &coins(u128::from(300_010_u128), "atom"),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert!(matches!(
            resp,
            ContractError::CoreError(CoreError::Std(StdError::Overflow { .. }))
        ));

        // Create a task that does cw20 transfer without attaching cw20 to the task
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
        let resp: ContractError = app
            .execute_contract(
                Addr::unchecked(user),
                contract_addr.clone(),
                &create_task_msg,
                &coins(u128::from(300_010_u128), "atom"),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert!(matches!(
                resp,
                ContractError::CoreError(CoreError::NotEnoughCw20 { lack, .. }) if lack == Uint128::from(10_u128)));
    }
}
