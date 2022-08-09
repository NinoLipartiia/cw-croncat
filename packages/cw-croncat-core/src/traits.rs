use crate::{
    error::CoreError,
    types::{BoundaryValidated, SlotType},
};
use cosmwasm_std::{Addr, Env};
use cw20::Balance;

pub trait GenericBalances {
    fn add_tokens(&mut self, add: Balance);
    fn minus_tokens(&mut self, minus: Balance);
}

pub trait FindAndMutate<'a, T, Rhs = &'a T>
where
    Self: IntoIterator<Item = T>,
{
    /// Safely adding and adding amount
    fn find_checked_add(&mut self, add: Rhs) -> Result<(), CoreError>;
    /// Safely finding and subtracting amount and remove it if it's zero
    fn find_checked_sub(&mut self, sub: Rhs) -> Result<(), CoreError>;
}

pub trait BalancesOperations<'a, T, Rhs> {
    fn checked_add_coins(&mut self, add: Rhs) -> Result<(), CoreError>;
    fn checked_sub_coins(&mut self, sub: Rhs) -> Result<(), CoreError>;
}

pub trait Intervals {
    fn next(&self, env: Env, boundary: BoundaryValidated) -> (u64, SlotType);
    fn is_valid(&self) -> bool;
}

pub trait TaskHash {
    fn to_hash(&self) -> String;
    fn to_hash_vec(&self) -> Vec<u8>;
    fn is_valid_msg(&self, self_addr: &Addr, sender: &Addr, owner_id: &Addr) -> bool;
    fn to_gas_total(&self) -> u64;
}
