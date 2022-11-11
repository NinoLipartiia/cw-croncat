use cosmwasm_std::{
    coin, coins,
    testing::{mock_env, mock_info},
    Addr, BlockInfo, DepsMut, Empty, Response,
};
use cw20::Cw20Coin;
use cw_croncat_core::{
    msg::InstantiateMsg,
    types::{BoundaryValidated, Interval, Task},
};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

use crate::{helpers::CwTemplateContract, ContractError, CwCroncat};

pub const AGENT0: &str = "cosmos1a7uhnpqthunr2rzj0ww0hwurpn42wyun6c5puz";
pub const AGENT1: &str = "cosmos17muvdgkep4ndptnyg38eufxsssq8jr3wnkysy8";
pub const AGENT2: &str = "cosmos1qxywje86amll9ptzxmla5ah52uvsd9f7drs2dl";
pub const AGENT3: &str = "cosmos1c3cy3wzzz3698ypklvh7shksvmefj69xhm89z2";
pub const AGENT4: &str = "cosmos1ykfcyj8fl6xzs88tsls05x93gmq68a7km05m4j";
pub const AGENT_BENEFICIARY: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
pub const ADMIN: &str = "cosmos1sjllsnramtg3ewxqwwrwjxfgc4n4ef9u0tvx7u";
pub const ANYONE: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
pub const PARTICIPANT0: &str = "cosmos1055rfv3fv0zxsp8h3x88mctnm7x9mlgmf4m4d6";
pub const PARTICIPANT1: &str = "cosmos1c3cy3wzzz3698ypklvh7shksvmefj69xhm89z2";
pub const PARTICIPANT2: &str = "cosmos1far5cqkvny7k9wq53aw0k42v3f76rcylzzv05n";
pub const PARTICIPANT3: &str = "cosmos1xj3xagnprtqpfnvyp7k393kmes73rpuxqgamd8";
pub const PARTICIPANT4: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
pub const PARTICIPANT5: &str = "cosmos1k5k7y4hgy5lkq0kj3k3e9k38lquh0m66kxsu5c";
pub const PARTICIPANT6: &str = "cosmos14a8clxc49z9e3mjzhamhkprt2hgf0y53zczzj0";
pub const VERY_RICH: &str = "cosmos1c3cy3wzzz3698ypklvh7shksvmefj69xhm89z2";
pub const NATIVE_DENOM: &str = "atom";
pub const TWO_MINUTES: u64 = 120_000_000_000;

pub fn mock_init(store: &CwCroncat, deps: DepsMut<Empty>) -> Result<Response, ContractError> {
    let msg = InstantiateMsg {
        denom: NATIVE_DENOM.to_string(),
        owner_id: None,
        gas_action_fee: None,
        gas_fraction: None,
        agent_nomination_duration: Some(360),
        cw_rules_addr: "todo".to_string(),
        gas_base_fee: None,
    };
    let info = mock_info("creator", &coins(1000, "meow"));
    store.instantiate(deps, mock_env(), info.clone(), msg)
}

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

pub fn cw_rules_template() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_rules::contract::execute,
        cw_rules::contract::instantiate,
        cw_rules::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    AppBuilder::new().build(|router, _, storage| {
        let accounts: Vec<(u128, String)> = vec![
            (6_000_000, ADMIN.to_string()),
            (500_000, ANYONE.to_string()),
            (2_000_000, AGENT0.to_string()),
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
            (u128::max_value(), VERY_RICH.to_string()),
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

pub fn proper_instantiate() -> (App, CwTemplateContract, Addr) {
    let mut app = mock_app();
    let cw_template_id = app.store_code(contract_template());
    let cw_rules_id = app.store_code(cw_rules_template());
    let owner_addr = Addr::unchecked(ADMIN);

    let cw_rules_addr = app
        .instantiate_contract(
            cw_rules_id,
            owner_addr.clone(),
            &cw_rules_core::msg::InstantiateMsg {},
            &[],
            "cw-rules",
            None,
        )
        .unwrap();
    let msg = InstantiateMsg {
        denom: NATIVE_DENOM.to_string(),
        cw_rules_addr: cw_rules_addr.to_string(),
        owner_id: Some(owner_addr.to_string()),
        gas_base_fee: None,
        gas_action_fee: None,
        gas_fraction: None,
        agent_nomination_duration: None,
    };
    let cw_template_contract_addr = app
        //Must send some available balance for rewards
        .instantiate_contract(
            cw_template_id,
            owner_addr.clone(),
            &msg,
            &coins(1, NATIVE_DENOM),
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

pub fn add_one_duration_of_time(block: &mut BlockInfo) {
    // block.time = block.time.plus_seconds(360);
    block.time = block.time.plus_seconds(420);
    block.height += 1;
}

pub fn add_1000_blocks(block: &mut BlockInfo) {
    // block.time = block.time.plus_seconds(360);
    block.time = block.time.plus_seconds(10);
    block.height += 1000;
}

pub fn default_task() -> Task {
    Task {
        owner_id: Addr::unchecked("bob"),
        interval: Interval::Once,
        boundary: BoundaryValidated {
            start: None,
            end: None,
        },
        funds_withdrawn_recurring: Default::default(),
        stop_on_fail: Default::default(),
        total_deposit: Default::default(),
        amount_for_one_task: Default::default(),
        actions: Default::default(),
        rules: Default::default(),
    }
}
