use cosmwasm_std::{to_binary, Addr, Binary, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, Executor};
use cw_rules_core::types::{CheckPassedProposals, CheckProposalStatus, Status};
use cwd_core::state::ProposalModule;
use cwd_voting::{
    multiple_choice::{
        MultipleChoiceOption, MultipleChoiceOptions, MultipleChoiceVote, VotingStrategy,
    },
    threshold::{PercentageThreshold, Threshold},
    voting::Vote,
};
use cwd_voting_cw20_staked::msg::ActiveThreshold;

use cw_rules_core::msg::{InstantiateMsg, QueryMsg, RuleResponse};

use crate::tests::helpers::{
    cw_rules_contract, multiple_proposal_contract, single_proposal_contract,
};

use super::helpers::{
    cw20_stake_contract, cw20_staked_balances_voting, cw20_template, cw_gov_contract, CREATOR_ADDR,
};

fn instantiate_with_staking_active_threshold(
    app: &mut App,
    proposal_module_code_id: u64,
    proposal_module_instantiate: Binary,
    initial_balances: Option<Vec<Cw20Coin>>,
    active_threshold: Option<ActiveThreshold>,
) -> Addr {
    let cw20_id = app.store_code(cw20_template());
    let cw20_staking_id = app.store_code(cw20_stake_contract());
    let governance_id = app.store_code(cw_gov_contract());
    let votemod_id = app.store_code(cw20_staked_balances_voting());

    let initial_balances = initial_balances.unwrap_or_else(|| {
        vec![Cw20Coin {
            address: CREATOR_ADDR.to_string(),
            amount: Uint128::new(100_000_000),
        }]
    });

    let governance_instantiate = cwd_core::msg::InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: cwd_interface::ModuleInstantiateInfo {
            code_id: votemod_id,
            msg: to_binary(&cwd_voting_cw20_staked::msg::InstantiateMsg {
                token_info: cwd_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token".to_string(),
                    name: "DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances,
                    marketing: None,
                    staking_code_id: cw20_staking_id,
                    unstaking_duration: None,
                    initial_dao_balance: None,
                },
                active_threshold,
            })
            .unwrap(),
            admin: Some(cwd_interface::Admin::CoreModule {}),
            label: "DAO DAO voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![cwd_interface::ModuleInstantiateInfo {
            code_id: proposal_module_code_id,
            msg: proposal_module_instantiate,
            admin: Some(cwd_interface::Admin::CoreModule {}),
            label: "DAO DAO governance module".to_string(),
        }],
        initial_items: None,
        dao_uri: None,
    };

    app.instantiate_contract(
        governance_id,
        Addr::unchecked(CREATOR_ADDR),
        &governance_instantiate,
        &[],
        "DAO DAO",
        None,
    )
    .unwrap()
}

#[test]
fn test_dao_single_proposal_ready() {
    let mut app = App::default();
    let code_id = app.store_code(cw_rules_contract());

    let instantiate = InstantiateMsg {};
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            &instantiate,
            &[],
            "cw-rules",
            None,
        )
        .unwrap();

    let proposal_module_code_id = app.store_code(single_proposal_contract());
    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate_govmod = cwd_proposal_single::msg::InstantiateMsg {
        threshold,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        close_proposal_on_execution_failure: true,
        pre_propose_info: cwd_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
    };
    let governance_addr = instantiate_with_staking_active_threshold(
        &mut app,
        proposal_module_code_id,
        to_binary(&instantiate_govmod).unwrap(),
        None,
        None,
    );
    let governance_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap().address;

    let govmod_config: cwd_proposal_single::state::Config = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cwd_proposal_single::msg::QueryMsg::Config {},
        )
        .unwrap();
    let dao = govmod_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake some tokens so we can propose
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(2000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Propose {
            title: "Cron".to_string(),
            description: "Cat".to_string(),
            msgs: vec![],
            proposer: None,
        },
        &[],
    )
    .unwrap();

    // It is not ready to execute yet, so false
    let res: RuleResponse<Option<Binary>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckProposalStatus(CheckProposalStatus {
                dao_address: govmod_single.to_string(),
                proposal_id: 1,
                status: Status::Passed,
            }),
        )
        .unwrap();
    assert_eq!(res, (false, None));

    // Approve proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    // It's now ready to be executed
    let res: RuleResponse<Option<Binary>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckProposalStatus(CheckProposalStatus {
                dao_address: govmod_single.to_string(),
                proposal_id: 1,
                status: Status::Passed,
            }),
        )
        .unwrap();
    assert_eq!(res, (true, None));

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    // It's executed now
    // Test if other types of status works
    let res: RuleResponse<Option<Binary>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::CheckProposalStatus(CheckProposalStatus {
                dao_address: govmod_single.to_string(),
                proposal_id: 1,
                status: Status::Executed,
            }),
        )
        .unwrap();
    assert_eq!(res, (true, None));
}

#[test]
fn test_dao_multiple_proposal_ready() {
    let mut app = App::default();
    let code_id = app.store_code(cw_rules_contract());
    let instantiate = InstantiateMsg {};
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            &instantiate,
            &[],
            "cw-rules",
            None,
        )
        .unwrap();

    let proposal_module_code_id = app.store_code(multiple_proposal_contract());
    let voting_strategy = VotingStrategy::SingleChoice {
        quorum: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate_govmod = cwd_proposal_multiple::msg::InstantiateMsg {
        voting_strategy,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        close_proposal_on_execution_failure: true,
        pre_propose_info: cwd_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
    };
    let governance_addr = instantiate_with_staking_active_threshold(
        &mut app,
        proposal_module_code_id,
        to_binary(&instantiate_govmod).unwrap(),
        None,
        None,
    );
    let governance_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap().address;

    let govmod_config: cwd_proposal_multiple::state::Config = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cwd_proposal_multiple::msg::QueryMsg::Config {},
        )
        .unwrap();
    let dao = govmod_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake some tokens so we can propose
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(2000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Propose {
            title: "Cron".to_string(),
            description: "Cat".to_string(),
            choices: MultipleChoiceOptions {
                options: vec![
                    MultipleChoiceOption {
                        description: "a".to_string(),
                        msgs: None,
                    },
                    MultipleChoiceOption {
                        description: "b".to_string(),
                        msgs: None,
                    },
                ],
            },
            proposer: None,
        },
        &[],
    )
    .unwrap();

    // It is not ready to execute yet, so false
    let res: RuleResponse<Option<Binary>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckProposalStatus(CheckProposalStatus {
                dao_address: govmod_single.to_string(),
                proposal_id: 1,
                status: Status::Passed,
            }),
        )
        .unwrap();
    assert_eq!(res, (false, None));

    // Approve proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: MultipleChoiceVote { option_id: 0 },
        },
        &[],
    )
    .unwrap();

    // It's now ready to be executed
    let res: RuleResponse<Option<Binary>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckProposalStatus(CheckProposalStatus {
                dao_address: govmod_single.to_string(),
                proposal_id: 1,
                status: Status::Passed,
            }),
        )
        .unwrap();
    assert_eq!(res, (true, None));

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    // It's executed now
    // Test if other types of status works
    let res: RuleResponse<Option<Binary>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::CheckProposalStatus(CheckProposalStatus {
                dao_address: govmod_single.to_string(),
                proposal_id: 1,
                status: Status::Executed,
            }),
        )
        .unwrap();
    assert_eq!(res, (true, None));
}

#[test]
fn test_dao_single_several_passed_proposals() {
    let mut app = App::default();
    let code_id = app.store_code(cw_rules_contract());

    let instantiate = InstantiateMsg {};
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            &instantiate,
            &[],
            "cw-rules",
            None,
        )
        .unwrap();

    let proposal_module_code_id = app.store_code(single_proposal_contract());
    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate_govmod = cwd_proposal_single::msg::InstantiateMsg {
        threshold,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        close_proposal_on_execution_failure: true,
        pre_propose_info: cwd_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
    };
    let governance_addr = instantiate_with_staking_active_threshold(
        &mut app,
        proposal_module_code_id,
        to_binary(&instantiate_govmod).unwrap(),
        None,
        None,
    );
    let governance_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap().address;

    let govmod_config: cwd_proposal_single::state::Config = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cwd_proposal_single::msg::QueryMsg::Config {},
        )
        .unwrap();
    let dao = govmod_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake some tokens so we can propose
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(2000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    // Create new proposals
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Propose {
            title: "Cron".to_string(),
            description: "Cat".to_string(),
            msgs: vec![],
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Propose {
            title: "Cron2".to_string(),
            description: "Cat2".to_string(),
            msgs: vec![],
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Propose {
            title: "Cron3".to_string(),
            description: "Cat3".to_string(),
            msgs: vec![],
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Propose {
            title: "Cron4".to_string(),
            description: "Cat4".to_string(),
            msgs: vec![],
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Propose {
            title: "Cron5".to_string(),
            description: "Cat5".to_string(),
            msgs: vec![],
            proposer: None,
        },
        &[],
    )
    .unwrap();

    // Neither proposal has passed
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));

    // Approve proposals 1, 3 and 4
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 3,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 4,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    // Now 1, 3 and 4 are passed and can be executed
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (true, vec![1, 3, 4]));

    // Execute proposals 1, 3, 4
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 3 },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 4 },
        &[],
    )
    .unwrap();

    // It's executed now
    // There shouldn't be any passed proposals
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));
}

#[test]
fn test_dao_multiple_several_passed_proposals() {
    let mut app = App::default();
    let code_id = app.store_code(cw_rules_contract());
    let instantiate = InstantiateMsg {};
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            &instantiate,
            &[],
            "cw-rules",
            None,
        )
        .unwrap();

    let proposal_module_code_id = app.store_code(multiple_proposal_contract());
    let voting_strategy = VotingStrategy::SingleChoice {
        quorum: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate_govmod = cwd_proposal_multiple::msg::InstantiateMsg {
        voting_strategy,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        close_proposal_on_execution_failure: true,
        pre_propose_info: cwd_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
    };
    let governance_addr = instantiate_with_staking_active_threshold(
        &mut app,
        proposal_module_code_id,
        to_binary(&instantiate_govmod).unwrap(),
        None,
        None,
    );
    let governance_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap().address;

    let govmod_config: cwd_proposal_multiple::state::Config = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cwd_proposal_multiple::msg::QueryMsg::Config {},
        )
        .unwrap();
    let dao = govmod_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake some tokens so we can propose
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(2000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    // Create five proposals
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Propose {
            title: "Cron".to_string(),
            description: "Cat".to_string(),
            choices: MultipleChoiceOptions {
                options: vec![
                    MultipleChoiceOption {
                        description: "a".to_string(),
                        msgs: None,
                    },
                    MultipleChoiceOption {
                        description: "b".to_string(),
                        msgs: None,
                    },
                ],
            },
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Propose {
            title: "Cron2".to_string(),
            description: "Cat2".to_string(),
            choices: MultipleChoiceOptions {
                options: vec![
                    MultipleChoiceOption {
                        description: "a".to_string(),
                        msgs: None,
                    },
                    MultipleChoiceOption {
                        description: "b".to_string(),
                        msgs: None,
                    },
                ],
            },
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Propose {
            title: "Cron3".to_string(),
            description: "Cat3".to_string(),
            choices: MultipleChoiceOptions {
                options: vec![
                    MultipleChoiceOption {
                        description: "a".to_string(),
                        msgs: None,
                    },
                    MultipleChoiceOption {
                        description: "b".to_string(),
                        msgs: None,
                    },
                ],
            },
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Propose {
            title: "Cron4".to_string(),
            description: "Cat4".to_string(),
            choices: MultipleChoiceOptions {
                options: vec![
                    MultipleChoiceOption {
                        description: "a".to_string(),
                        msgs: None,
                    },
                    MultipleChoiceOption {
                        description: "b".to_string(),
                        msgs: None,
                    },
                ],
            },
            proposer: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Propose {
            title: "Cron5".to_string(),
            description: "Cat5".to_string(),
            choices: MultipleChoiceOptions {
                options: vec![
                    MultipleChoiceOption {
                        description: "a".to_string(),
                        msgs: None,
                    },
                    MultipleChoiceOption {
                        description: "b".to_string(),
                        msgs: None,
                    },
                ],
            },
            proposer: None,
        },
        &[],
    )
    .unwrap();

    // Neither proposal has passed
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));

    // Vote on proposals 1, 3 and 4
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: MultipleChoiceVote { option_id: 0 },
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Vote {
            proposal_id: 3,
            vote: MultipleChoiceVote { option_id: 1 },
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Vote {
            proposal_id: 4,
            vote: MultipleChoiceVote { option_id: 0 },
        },
        &[],
    )
    .unwrap();

    // Proposals 1, 3 and 4 have passed status
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (true, vec![1, 3, 4]));

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Execute { proposal_id: 3 },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cwd_proposal_multiple::msg::ExecuteMsg::Execute { proposal_id: 4 },
        &[],
    )
    .unwrap();

    // It's executed now
    // There shouldn't be any passed proposals
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));
}

#[test]
fn test_dao_single_many_passed_proposals() {
    let mut app = App::default();
    let code_id = app.store_code(cw_rules_contract());

    let instantiate = InstantiateMsg {};
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            &instantiate,
            &[],
            "cw-rules",
            None,
        )
        .unwrap();

    let proposal_module_code_id = app.store_code(single_proposal_contract());
    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate_govmod = cwd_proposal_single::msg::InstantiateMsg {
        threshold,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        close_proposal_on_execution_failure: true,
        pre_propose_info: cwd_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
    };
    let governance_addr = instantiate_with_staking_active_threshold(
        &mut app,
        proposal_module_code_id,
        to_binary(&instantiate_govmod).unwrap(),
        None,
        None,
    );
    let governance_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap().address;

    let govmod_config: cwd_proposal_single::state::Config = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cwd_proposal_single::msg::QueryMsg::Config {},
        )
        .unwrap();
    let dao = govmod_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake some tokens so we can propose
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(2000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    // Create new proposals
    for num in 1..101 {
        let mut title = "Cron".to_string();
        title.push(num.into());
        let mut description = "Cat".to_string();
        description.push(num.into());
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &cwd_proposal_single::msg::ExecuteMsg::Propose {
                title,
                description,
                msgs: vec![],
                proposer: None,
            },
            &[],
        )
        .unwrap();
    }

    // Neither proposal has passed
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));

    // Approve even proposals
    let mut passed_ids = vec![];
    for num in 1..51 {
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &cwd_proposal_single::msg::ExecuteMsg::Vote {
                proposal_id: 2 * num as u64,
                vote: Vote::Yes,
            },
            &[],
        )
        .unwrap();
        passed_ids.push(2 * num as u64)
    }

    // Now all even proposals are passed and can be executed
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (true, passed_ids));

    // Execute even proposals
    for num in 1..51 {
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &cwd_proposal_single::msg::ExecuteMsg::Execute {
                proposal_id: 2 * num as u64,
            },
            &[],
        )
        .unwrap();
    }

    // It's executed now
    // There shouldn't be any passed proposals
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));
}

#[test]
fn test_dao_multiple_many_passed_proposals() {
    let mut app = App::default();
    let code_id = app.store_code(cw_rules_contract());
    let instantiate = InstantiateMsg {};
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            &instantiate,
            &[],
            "cw-rules",
            None,
        )
        .unwrap();

    let proposal_module_code_id = app.store_code(multiple_proposal_contract());
    let voting_strategy = VotingStrategy::SingleChoice {
        quorum: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate_govmod = cwd_proposal_multiple::msg::InstantiateMsg {
        voting_strategy,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        close_proposal_on_execution_failure: true,
        pre_propose_info: cwd_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
    };
    let governance_addr = instantiate_with_staking_active_threshold(
        &mut app,
        proposal_module_code_id,
        to_binary(&instantiate_govmod).unwrap(),
        None,
        None,
    );
    let governance_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap().address;

    let govmod_config: cwd_proposal_multiple::state::Config = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cwd_proposal_multiple::msg::QueryMsg::Config {},
        )
        .unwrap();
    let dao = govmod_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake some tokens so we can propose
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(2000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    // Create five proposals
    for num in 1..101 {
        let mut title = "Cron".to_string();
        title.push(num.into());
        let mut description = "Cat".to_string();
        description.push(num.into());
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &cwd_proposal_multiple::msg::ExecuteMsg::Propose {
                title,
                description,
                choices: MultipleChoiceOptions {
                    options: vec![
                        MultipleChoiceOption {
                            description: "a".to_string(),
                            msgs: None,
                        },
                        MultipleChoiceOption {
                            description: "b".to_string(),
                            msgs: None,
                        },
                    ],
                },
                proposer: None,
            },
            &[],
        )
        .unwrap();
    }

    // Neither proposal has passed
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));

    // Vote on even proposals
    let mut passed_ids = vec![];
    for num in 1..51 {
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &cwd_proposal_multiple::msg::ExecuteMsg::Vote {
                proposal_id: 2 * num as u64,
                vote: MultipleChoiceVote { option_id: 0 },
            },
            &[],
        )
        .unwrap();
        passed_ids.push(2 * num as u64)
    }

    // Proposals 2, 4, 6, ... 98, 100 have passed status
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (true, passed_ids));

    // Execute passed proposals
    for num in 1..51 {
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &cwd_proposal_multiple::msg::ExecuteMsg::Execute {
                proposal_id: 2 * num as u64,
            },
            &[],
        )
        .unwrap();
    }

    // It's executed now
    // There shouldn't be any passed proposals
    let res: RuleResponse<Vec<u64>> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::CheckPassedProposals(CheckPassedProposals {
                dao_address: govmod_single.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(res, (false, vec![]));
}
