use crate::error::ContractError;
use crate::helpers::Task;
use crate::CwCroncat;
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{coins, Addr, BankMsg, CosmosMsg, Order, StdResult};
use cw_croncat_core::types::{Action, BoundaryValidated, Interval};
use cw_storage_plus::Bound;

#[test]
fn check_task_storage_structure() -> StdResult<()> {
    let mut storage = MockStorage::new();
    let store = CwCroncat::default();

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
        total_deposit: Default::default(),
        amount_for_one_task: Default::default(),
        actions: vec![Action {
            msg,
            gas_limit: Some(150_000),
        }],
        rules: None,
    };
    let task_id_str = "69217dd2b6334abe2544a12fcb89588f9cc5c62a298b8720706d9befa3d736d3";
    let task_id = task_id_str.to_string().into_bytes();

    // create a task
    let res = store
        .tasks
        .update(&mut storage, &task.to_hash_vec(), |old| match old {
            Some(_) => Err(ContractError::CustomError {
                val: "Already exists".to_string(),
            }),
            None => Ok(task.clone()),
        });
    assert_eq!(res.unwrap(), task.clone());

    // get task ids by owner
    let task_ids_by_owner: Vec<String> = store
        .tasks
        .idx
        .owner
        .prefix(Addr::unchecked("nobody".to_string()))
        .keys(&mut storage, None, None, Order::Ascending)
        .take(5)
        .map(|x| x.map(|addr| addr.to_string()))
        .collect::<StdResult<Vec<_>>>()?;
    assert_eq!(task_ids_by_owner, vec![task_id_str.clone()]);

    // get all task ids
    let all_task_ids: StdResult<Vec<String>> = store
        .tasks
        .range(&mut storage, None, None, Order::Ascending)
        .take(10)
        .map(|x| x.map(|(_, task)| task.to_hash()))
        .collect();
    assert_eq!(all_task_ids.unwrap(), vec![task_id_str.clone()]);

    // get single task
    let get_task = store.tasks.load(&mut storage, &task_id)?;
    assert_eq!(get_task, task);

    Ok(())
}

// test for range / Ordered time slots
#[test]
fn check_slots_storage_structure() -> StdResult<()> {
    let mut storage = MockStorage::new();
    let store = CwCroncat::default();

    let task_id_str = "3ccb739ea050ebbd2e08f74aeb0b7aa081b15fa78504cba44155ec774452bbee";
    let task_id = task_id_str.to_string().into_bytes();
    let tasks_vec = vec![task_id];

    store
        .time_slots
        .save(&mut storage, 12345 as u64, &tasks_vec.clone())?;
    store
        .time_slots
        .save(&mut storage, 12346 as u64, &tasks_vec.clone())?;
    store
        .time_slots
        .save(&mut storage, 22345 as u64, &tasks_vec.clone())?;

    // get all under one key
    let all_slots_res: StdResult<Vec<_>> = store
        .time_slots
        .range(&mut storage, None, None, Order::Ascending)
        .take(5)
        .collect();
    let all_slots = all_slots_res?;
    assert_eq!(all_slots[0].0, 12345);
    assert_eq!(all_slots[1].0, 12346);
    assert_eq!(all_slots[2].0, 22345);

    // Range test
    let range_slots: StdResult<Vec<_>> = store
        .time_slots
        .range(
            &mut storage,
            Some(Bound::exclusive(12345 as u64)),
            Some(Bound::inclusive(22346 as u64)),
            Order::Descending,
        )
        .collect();
    let slots = range_slots?;
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].0, 22345);
    assert_eq!(slots[1].0, 12346);

    Ok(())
}
