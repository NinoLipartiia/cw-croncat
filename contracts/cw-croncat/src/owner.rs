use std::slice;

use crate::error::ContractError;
use crate::helpers::has_cw_coins;
use crate::state::{Config, CwCroncat};
use cosmwasm_std::{
    has_coins, to_binary, Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, WasmMsg,
};
use cw20::{Balance, Cw20ExecuteMsg};
use cw_croncat_core::msg::{
    ExecuteMsg, GetBalancesResponse, GetConfigResponse, GetWalletBalancesResponse,
};

impl<'a> CwCroncat<'a> {
    pub(crate) fn query_config(&self, deps: Deps) -> StdResult<GetConfigResponse> {
        let c: Config = self.config.load(deps.storage)?;
        Ok(GetConfigResponse {
            paused: c.paused,
            owner_id: c.owner_id,
            // treasury_id: c.treasury_id,
            min_tasks_per_agent: c.min_tasks_per_agent,
            agent_active_indices: c.agent_active_indices,
            agents_eject_threshold: c.agents_eject_threshold,
            native_denom: c.native_denom,
            agent_fee: c.agent_fee,
            gas_price: c.gas_price,
            proxy_callback_gas: c.proxy_callback_gas,
            slot_granularity: c.slot_granularity,
        })
    }

    pub(crate) fn query_balances(&self, deps: Deps) -> StdResult<GetBalancesResponse> {
        let c: Config = self.config.load(deps.storage)?;
        Ok(GetBalancesResponse {
            native_denom: c.native_denom,
            available_balance: c.available_balance,
            staked_balance: c.staked_balance,
            cw20_whitelist: c.cw20_whitelist,
        })
    }

    /// Returns user cw20 balances locked inside this contract
    pub(crate) fn query_wallet_balances(
        &self,
        deps: Deps,
        wallet: String,
    ) -> StdResult<GetWalletBalancesResponse> {
        let addr = deps.api.addr_validate(&wallet)?;
        let balances = self.balances.may_load(deps.storage, &addr)?;
        Ok(GetWalletBalancesResponse {
            cw20_balances: balances.unwrap_or_default(),
        })
    }

    /// Changes core configurations
    /// Should only be updated by owner -- in best case DAO based :)
    pub fn update_settings(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        payload: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        for coin in info.funds.iter() {
            if coin.amount.u128() > 0 {
                return Err(ContractError::AttachedDeposit {});
            }
        }
        match payload {
            ExecuteMsg::UpdateSettings {
                owner_id,
                slot_granularity,
                paused,
                agent_fee,
                gas_price,
                proxy_callback_gas,
                min_tasks_per_agent,
                agents_eject_threshold,
                // treasury_id,
            } => {
                self.config
                    .update(deps.storage, |mut config| -> Result<_, ContractError> {
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
                        if let Some(min_tasks_per_agent) = min_tasks_per_agent {
                            config.min_tasks_per_agent = min_tasks_per_agent;
                        }
                        if let Some(agents_eject_threshold) = agents_eject_threshold {
                            config.agents_eject_threshold = agents_eject_threshold;
                        }
                        Ok(config)
                    })?;
            }
            _ => unreachable!(),
        }
        let c: Config = self.config.load(deps.storage)?;
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
            .add_attribute("min_tasks_per_agent", c.min_tasks_per_agent.to_string())
            .add_attribute(
                "agent_active_indices",
                c.agent_active_indices
                    .iter()
                    .map(|a| format!("{:?}.{}", a.0, a.1))
                    .collect::<String>(),
            )
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
        &self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        balances: Vec<Balance>,
        account_id: Addr,
    ) -> Result<Response, ContractError> {
        let mut config = self.config.load(deps.storage)?;

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
                        config.available_balance.checked_sub_native(&bal)?;
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
                            .checked_sub_cw20(slice::from_ref(&bal))?;

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
        self.config.save(deps.storage, &config)?;

        Ok(Response::new()
            .add_attribute("method", "move_balance")
            .add_attribute("account_id", account_id.to_string())
            .add_submessages(messages.unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::ContractError;
    use crate::state::CwCroncat;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, Addr, MessageInfo};
    use cw20::Balance;
    use cw_croncat_core::msg::{
        ExecuteMsg, GetBalancesResponse, GetConfigResponse, InstantiateMsg, QueryMsg,
    };

    #[test]
    fn update_settings() {
        let mut deps = mock_dependencies_with_balance(&coins(200, ""));
        let mut store = CwCroncat::default();

        let msg = InstantiateMsg {
            denom: "atom".to_string(),
            owner_id: None,
            gas_base_fee: None,
            agent_nomination_duration: Some(360),
        };
        let info = MessageInfo {
            sender: Addr::unchecked("creator"),
            funds: vec![],
        };
        mock_info("creator", &coins(0, "meow"));
        let res_init = store
            .instantiate(deps.as_mut(), mock_env(), info.clone(), msg)
            .unwrap();
        assert_eq!(0, res_init.messages.len());

        let payload = ExecuteMsg::UpdateSettings {
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

        // non-owner fails
        let unauth_info = MessageInfo {
            sender: Addr::unchecked("michael_scott"),
            funds: vec![],
        };
        let res_fail = store.execute(deps.as_mut(), mock_env(), unauth_info, payload.clone());
        match res_fail {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // non-zero deposit fails
        let with_deposit_info = mock_info("owner_id", &coins(1000, "meow"));
        let res_fail = store.execute(
            deps.as_mut(),
            mock_env(),
            with_deposit_info,
            payload.clone(),
        );
        match res_fail {
            Err(ContractError::AttachedDeposit {}) => {}
            _ => panic!("Must return deposit error"),
        }

        // do the right thing
        let res_exec = store
            .execute(deps.as_mut(), mock_env(), info.clone(), payload)
            .unwrap();
        assert_eq!(0, res_exec.messages.len());

        // it worked, let's query the state
        let res = store
            .query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {})
            .unwrap();
        let value: GetConfigResponse = from_binary(&res).unwrap();
        assert_eq!(true, value.paused);
        assert_eq!(info.sender, value.owner_id);
    }

    #[test]
    fn move_balances_auth_checks() {
        let mut deps = mock_dependencies_with_balance(&coins(200000000, "atom"));
        let mut store = CwCroncat::default();
        let info = mock_info("owner_id", &coins(1000, "meow"));
        let unauth_info = mock_info("michael_scott", &coins(2, "shrute_bucks"));
        let exist_bal = vec![Balance::from(coins(2, "atom"))];
        let non_exist_bal = vec![Balance::from(coins(2, "shrute_bucks"))];

        // instantiate with owner, then add treasury
        let msg = InstantiateMsg {
            denom: "atom".to_string(),
            owner_id: None,
            gas_base_fee: None,
            agent_nomination_duration: Some(360),
        };
        let res_init = store
            .instantiate(deps.as_mut(), mock_env(), info.clone(), msg)
            .unwrap();
        assert!(res_init.messages.is_empty());

        let payload = ExecuteMsg::UpdateSettings {
            paused: None,
            owner_id: None,
            // treasury_id: Some(Addr::unchecked("money_bags")),
            agent_fee: None,
            min_tasks_per_agent: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
        };
        let info_setting = mock_info("owner_id", &coins(0, "meow"));
        let res_exec = store
            .execute(deps.as_mut(), mock_env(), info_setting, payload)
            .unwrap();
        assert!(res_exec.messages.is_empty());

        // try to move funds as non-owner
        let msg_move_1 = ExecuteMsg::MoveBalances {
            balances: non_exist_bal,
            account_id: Addr::unchecked("scammer"),
        };
        let res_fail_1 = store.execute(deps.as_mut(), mock_env(), unauth_info, msg_move_1);
        match res_fail_1 {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // try to move funds to account other than treasury or owner
        let msg_move_2 = ExecuteMsg::MoveBalances {
            balances: exist_bal.clone(),
            account_id: Addr::unchecked("scammer"),
        };
        let res_fail_2 = store.execute(deps.as_mut(), mock_env(), info.clone(), msg_move_2);
        match res_fail_2 {
            Err(ContractError::CustomError { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn move_balances_native() {
        let mut deps = mock_dependencies_with_balance(&coins(200000000, "atom"));
        let mut store = CwCroncat::default();
        let info = mock_info("owner_id", &coins(1000, "meow"));
        let exist_bal = vec![Balance::from(coins(2, "atom"))];
        let spensive_bal = vec![Balance::from(coins(2000000000000, "atom"))];
        let money_bags = Addr::unchecked("owner_id");

        // instantiate with owner, then add treasury
        let msg = InstantiateMsg {
            denom: "atom".to_string(),
            owner_id: None,
            gas_base_fee: None,
            agent_nomination_duration: Some(360),
        };
        let res_init = store
            .instantiate(deps.as_mut(), mock_env(), info.clone(), msg)
            .unwrap();
        assert!(res_init.messages.is_empty());

        let payload = ExecuteMsg::UpdateSettings {
            paused: None,
            owner_id: None,
            // treasury_id: Some(money_bags.clone()),
            agent_fee: None,
            min_tasks_per_agent: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
        };
        let info_settings = mock_info("owner_id", &coins(0, "meow"));
        let res_exec = store
            .execute(deps.as_mut(), mock_env(), info_settings, payload)
            .unwrap();
        assert!(res_exec.messages.is_empty());

        // try to move funds with greater amount than native available
        let msg_move_fail = ExecuteMsg::MoveBalances {
            balances: spensive_bal,
            account_id: money_bags.clone(),
        };
        let res_fail = store.execute(deps.as_mut(), mock_env(), info.clone(), msg_move_fail);
        match res_fail {
            Err(ContractError::CustomError { .. }) => {}
            _ => panic!("Must return custom not enough funds error"),
        }

        // try to move native available funds
        let msg_move = ExecuteMsg::MoveBalances {
            balances: exist_bal,
            account_id: money_bags,
        };
        let res_exec = store
            .execute(deps.as_mut(), mock_env(), info.clone(), msg_move)
            .unwrap();
        assert!(!res_exec.messages.is_empty());

        // it worked, let's query the state
        let res_bal = store
            .query(deps.as_ref(), mock_env(), QueryMsg::GetBalances {})
            .unwrap();
        let balances: GetBalancesResponse = from_binary(&res_bal).unwrap();
        assert_eq!(
            vec![coin(199999998, "atom"), coin(1000, "meow")],
            balances.available_balance.native
        );
    }

    // // TODO: Setup CW20 logic / balances!
    // #[test]
    // fn move_balances_cw() {
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
