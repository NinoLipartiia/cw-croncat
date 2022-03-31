use crate::error::ContractError;
use crate::helpers::has_cw_coins;
use crate::msg::{BalancesResponse, ConfigResponse, ExecuteMsg};
use crate::state::{Config, CONFIG};
use cosmwasm_std::{
    has_coins, to_binary, Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, WasmMsg,
};
use cw20::{Balance, Cw20ExecuteMsg};

pub(crate) fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let c: Config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        paused: c.paused,
        owner_id: c.owner_id,
        // treasury_id: c.treasury_id,
        agent_task_ratio: c.agent_task_ratio,
        agent_active_index: c.agent_active_index,
        agents_eject_threshold: c.agents_eject_threshold,
        native_denom: c.native_denom,
        agent_fee: c.agent_fee,
        gas_price: c.gas_price,
        proxy_callback_gas: c.proxy_callback_gas,
        slot_granularity: c.slot_granularity,
    })
}

pub(crate) fn query_balances(deps: Deps) -> StdResult<BalancesResponse> {
    let c: Config = CONFIG.load(deps.storage)?;
    Ok(BalancesResponse {
        native_denom: c.native_denom,
        available_balance: c.available_balance,
        staked_balance: c.staked_balance,
        cw20_whitelist: c.cw20_whitelist,
    })
}

/// Changes core configurations
/// Should only be updated by owner -- in best case DAO based :)
pub fn update_settings(
    deps: DepsMut,
    info: MessageInfo,
    payload: ExecuteMsg,
) -> Result<Response, ContractError> {
    match payload {
        ExecuteMsg::UpdateSettings {
            owner_id,
            slot_granularity,
            paused,
            agent_fee,
            gas_price,
            proxy_callback_gas,
            agent_task_ratio,
            agents_eject_threshold,
            // treasury_id,
        } => {
            CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
                if info.sender != config.owner_id {
                    return Err(ContractError::Unauthorized {});
                }

                if let Some(owner_id) = owner_id {
                    config.owner_id = owner_id;
                }
                // if let Some(treasury_id) = treasury_id {
                //     config.treasury_id = Some(treasury_id);
                // }

                if let Some(slot_granularity) = slot_granularity {
                    config.slot_granularity = slot_granularity;
                }
                if let Some(paused) = paused {
                    config.paused = paused;
                }
                if let Some(gas_price) = gas_price {
                    config.gas_price = gas_price;
                }
                if let Some(proxy_callback_gas) = proxy_callback_gas {
                    config.proxy_callback_gas = proxy_callback_gas;
                }
                if let Some(agent_fee) = agent_fee {
                    config.agent_fee = agent_fee;
                }
                if let Some(agent_task_ratio) = agent_task_ratio {
                    config.agent_task_ratio = [agent_task_ratio[0], agent_task_ratio[1]];
                }
                if let Some(agents_eject_threshold) = agents_eject_threshold {
                    config.agents_eject_threshold = agents_eject_threshold;
                }
                Ok(config)
            })?;
        }
        _ => unreachable!(),
    }
    let c: Config = CONFIG.load(deps.storage)?;
    Ok(Response::new()
        .add_attribute("method", "update_settings")
        .add_attribute("paused", c.paused.to_string())
        .add_attribute("owner_id", c.owner_id.to_string())
        // .add_attribute(
        //     "treasury_id",
        //     c.treasury_id
        //         .unwrap_or_else(|| Addr::unchecked(""))
        //         .to_string(),
        // )
        .add_attribute(
            "agent_task_ratio",
            c.agent_task_ratio
                .iter()
                .copied()
                .map(|i| i.to_string())
                .collect::<String>(),
        )
        .add_attribute("agent_active_index", c.agent_active_index.to_string())
        .add_attribute(
            "agents_eject_threshold",
            c.agents_eject_threshold.to_string(),
        )
        .add_attribute("native_denom", c.native_denom)
        .add_attribute("agent_fee", c.agent_fee.to_string())
        .add_attribute("gas_price", c.gas_price.to_string())
        .add_attribute("proxy_callback_gas", c.proxy_callback_gas.to_string())
        .add_attribute("slot_granularity", c.slot_granularity.to_string()))
}

/// Move Balance
/// Allows owner to move balance to DAO or to let treasury transfer to itself only.
/// This is a restricted method for moving funds utilized in growth management strategies.
pub fn move_balances(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    balances: Vec<Balance>,
    account_id: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // // Check if is owner OR the treasury account making the transfer request
    // if let Some(treasury_id) = config.treasury_id.clone() {
    //     if treasury_id != info.sender && config.owner_id != info.sender {
    //         return Err(ContractError::Unauthorized {});
    //     }
    // } else 
    if info.sender != config.owner_id {
        return Err(ContractError::Unauthorized {});
    }

    // for now, only allow movement of funds between owner and treasury
    // let check_account = config
    //     .treasury_id
    //     .clone()
    //     .unwrap_or_else(|| config.owner_id.clone());
    let check_account = config.owner_id.clone();
    if check_account != account_id && config.owner_id != account_id {
        return Err(ContractError::CustomError {
            val: "Cannot move funds to this account".to_string(),
        });
    }

    // Querier guarantees to returns up-to-date data, including funds sent in this handle message
    // https://github.com/CosmWasm/wasmd/blob/master/x/wasm/internal/keeper/keeper.go#L185-L192
    let state_balances = deps.querier.query_all_balances(&env.contract.address)?;
    let mut has_fund_err = false;

    let messages: Result<Vec<SubMsg>, ContractError> = balances
        .iter()
        .map(|balance| -> Result<SubMsg<_>, ContractError> {
            match balance {
                Balance::Native(balance) => {
                    // check has enough
                    let bal = balance.clone().into_vec();
                    let has_c = has_coins(&state_balances, &bal[0]);
                    if !has_c {
                        has_fund_err = true;
                        // TODO: refactor to not need
                        return Ok(SubMsg::new(BankMsg::Send {
                            to_address: account_id.clone().into(),
                            amount: vec![Coin::new(0, "")],
                        }));
                    }

                    // Update internal registry balance
                    config
                        .available_balance
                        .minus_tokens(Balance::from(bal.clone()));
                    Ok(SubMsg::new(BankMsg::Send {
                        to_address: account_id.clone().into(),
                        amount: bal,
                    }))
                }
                Balance::Cw20(token) => {
                    // check has enough
                    let bal = token.clone();
                    if !has_cw_coins(&config.available_balance.cw20, &bal) {
                        has_fund_err = true;
                        // TODO: refactor to not need
                        return Ok(SubMsg::new(BankMsg::Send {
                            to_address: account_id.clone().into(),
                            amount: vec![Coin::new(0, "")],
                        }));
                    }

                    // Update internal registry balance
                    config
                        .available_balance
                        .minus_tokens(Balance::from(bal.clone()));

                    let msg = Cw20ExecuteMsg::Transfer {
                        recipient: account_id.clone().into(),
                        amount: bal.amount,
                    };
                    Ok(SubMsg::new(WasmMsg::Execute {
                        contract_addr: bal.address.to_string(),
                        msg: to_binary(&msg)?,
                        funds: vec![],
                    }))
                }
            }
        })
        .collect();

    // failed
    if has_fund_err {
        return Err(ContractError::CustomError {
            val: "Not enough funds".to_string(),
        });
    }

    // Update balances in config
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "move_balance")
        .add_attribute("account_id", account_id.to_string())
        .add_submessages(messages.unwrap()))
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::error::ContractError;
    use crate::msg::{BalancesResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, Addr};
    use cw20::Balance;

    #[test]
    fn test_update_settings() {
        let mut deps = mock_dependencies_with_balance(&coins(200, ""));

        let msg = InstantiateMsg { owner_id: None };
        let info = mock_info("creator", &coins(1000, "meow"));
        let res_init = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res_init.messages.len());

        let payload = ExecuteMsg::UpdateSettings {
            paused: Some(true),
            owner_id: None,
            // treasury_id: None,
            agent_fee: None,
            agent_task_ratio: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
        };

        // non-owner fails
        let unauth_info = mock_info("michael_scott", &coins(2, "shrute_bucks"));
        let res_fail = execute(deps.as_mut(), mock_env(), unauth_info, payload.clone());
        match res_fail {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // do the right thing
        let res_exec = execute(deps.as_mut(), mock_env(), info.clone(), payload).unwrap();
        assert_eq!(0, res_exec.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(true, value.paused);
        assert_eq!(info.sender, value.owner_id);
    }

    #[test]
    fn test_owner_move_balances_auth_checks() {
        let mut deps = mock_dependencies_with_balance(&coins(200000000, "atom"));
        let info = mock_info("owner_id", &coins(1000, "meow"));
        let unauth_info = mock_info("michael_scott", &coins(2, "shrute_bucks"));
        let exist_bal = vec![Balance::from(coins(2, "atom"))];
        let non_exist_bal = vec![Balance::from(coins(2, "shrute_bucks"))];

        // instantiate with owner, then add treasury
        let msg = InstantiateMsg { owner_id: None };
        let res_init = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert!(res_init.messages.is_empty());

        let payload = ExecuteMsg::UpdateSettings {
            paused: None,
            owner_id: None,
            // treasury_id: Some(Addr::unchecked("money_bags")),
            agent_fee: None,
            agent_task_ratio: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
        };
        let res_exec = execute(deps.as_mut(), mock_env(), info.clone(), payload).unwrap();
        assert!(res_exec.messages.is_empty());

        // try to move funds as non-owner
        let msg_move_1 = ExecuteMsg::MoveBalances {
            balances: non_exist_bal,
            account_id: Addr::unchecked("scammer"),
        };
        let res_fail_1 = execute(deps.as_mut(), mock_env(), unauth_info, msg_move_1);
        match res_fail_1 {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // try to move funds to account other than treasury or owner
        let msg_move_2 = ExecuteMsg::MoveBalances {
            balances: exist_bal.clone(),
            account_id: Addr::unchecked("scammer"),
        };
        let res_fail_2 = execute(deps.as_mut(), mock_env(), info.clone(), msg_move_2);
        match res_fail_2 {
            Err(ContractError::CustomError { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn test_owner_move_balances_native() {
        let mut deps = mock_dependencies_with_balance(&coins(200000000, "atom"));
        let info = mock_info("owner_id", &coins(1000, "meow"));
        let exist_bal = vec![Balance::from(coins(2, "atom"))];
        let spensive_bal = vec![Balance::from(coins(2000000000000, "atom"))];
        let money_bags = Addr::unchecked("owner_id");

        // instantiate with owner, then add treasury
        let msg = InstantiateMsg { owner_id: None };
        let res_init = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert!(res_init.messages.is_empty());

        let payload = ExecuteMsg::UpdateSettings {
            paused: None,
            owner_id: None,
            // treasury_id: Some(money_bags.clone()),
            agent_fee: None,
            agent_task_ratio: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
        };
        let res_exec = execute(deps.as_mut(), mock_env(), info.clone(), payload).unwrap();
        assert!(res_exec.messages.is_empty());

        // try to move funds with greater amount than native available
        let msg_move_fail = ExecuteMsg::MoveBalances {
            balances: spensive_bal,
            account_id: money_bags.clone(),
        };
        let res_fail = execute(deps.as_mut(), mock_env(), info.clone(), msg_move_fail);
        match res_fail {
            Err(ContractError::CustomError { .. }) => {}
            _ => panic!("Must return custom not enough funds error"),
        }

        // try to move native available funds
        let msg_move = ExecuteMsg::MoveBalances {
            balances: exist_bal,
            account_id: money_bags,
        };
        let res_exec = execute(deps.as_mut(), mock_env(), info.clone(), msg_move).unwrap();
        assert!(!res_exec.messages.is_empty());

        // it worked, let's query the state
        let res_bal = query(deps.as_ref(), mock_env(), QueryMsg::GetBalances {}).unwrap();
        let balances: BalancesResponse = from_binary(&res_bal).unwrap();
        assert_eq!(
            vec![coin(199999998, "atom"), coin(1000, "meow")],
            balances.available_balance.native
        );
    }

    // // TODO: Setup CW20 logic / balances!
    // #[test]
    // fn test_owner_move_balances_cw() {
    //     let mut deps = mock_dependencies_with_balance(&coins(200000000, "atom"));
    //     let info = mock_info("owner_id", &vec![Balance::Cw20(1000, "meow")]);
    //     let money_bags = Addr::unchecked("money_bags");
    //     let exist_bal = vec![Balance::from(coins(2, "atom"))];
    //     let spensive_bal = vec![Balance::from(coins(2000000000000, "atom"))];
    //     let non_exist_bal = vec![Balance::from(coins(2, "shrute_bucks"))];

    //     // instantiate with owner, then add treasury
    //     let msg = InstantiateMsg { owner_id: None };
    //     let res_init = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    //     assert!(res_init.messages.is_empty());

    //     let payload = ExecuteMsg::UpdateSettings {
    //         paused: None,
    //         owner_id: None,
    //         treasury_id: Some(money_bags.clone()),
    //         agent_fee: None,
    //         agent_task_ratio: None,
    //         agents_eject_threshold: None,
    //         gas_price: None,
    //         proxy_callback_gas: None,
    //         slot_granularity: None,
    //     };
    //     let res_exec = execute(deps.as_mut(), mock_env(), info.clone(), payload).unwrap();
    //     assert!(res_exec.messages.is_empty());

    //     // try to move funds with greater amount than cw available
    //     let msg_move_fail = ExecuteMsg::MoveBalances { balances: spensive_bal, account_id: money_bags.clone() };
    //     let res_fail = execute(deps.as_mut(), mock_env(), info.clone(), msg_move_fail);
    //     match res_fail {
    //         Err(ContractError::CustomError { .. }) => {}
    //         _ => panic!("Must return custom not enough funds error"),
    //     }

    //     // try to move cw available funds
    //     // // do the right thing
    //     // let res_exec = execute(deps.as_mut(), mock_env(), info.clone(), payload).unwrap();
    //     // assert!(!res_exec.messages.is_empty());

    //     // // it worked, let's query the state
    //     // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
    //     // let value: ConfigResponse = from_binary(&res).unwrap();
    //     // println!("CONFIG {:?}", value);
    //     // assert_eq!(true, value.paused);
    //     // assert_eq!(info.sender, value.owner_id);
    // }
}
