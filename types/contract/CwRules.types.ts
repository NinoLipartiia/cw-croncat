/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.14.2.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export type CheckOwnerOfNftResponse = [boolean, Binary | null];
export type Binary = string;
export interface CheckOwnerOfNft {
  address: string;
  nft_address: string;
  token_id: string;
  [k: string]: unknown;
}
export type CheckProposalStatusResponse = [boolean, Binary | null];
export type Status = "execution_failed" | "open" | "rejected" | "passed" | "executed" | "closed";
export interface CheckProposalStatus {
  dao_address: string;
  proposal_id: number;
  status: Status;
  [k: string]: unknown;
}
export type ExecuteMsg = {
  query_result: {
    [k: string]: unknown;
  };
};
export type GenericQueryResponse = [boolean, Binary | null];
export type GetBalanceResponse = [boolean, Binary | null];
export type GetCw20BalanceResponse = [boolean, Binary | null];
export type HasBalanceGteResponse = [boolean, Binary | null];
export type Balance = {
  native: NativeBalance;
} | {
  cw20: Cw20CoinVerified;
};
export type Uint128 = string;
export type NativeBalance = Coin[];
export type Addr = string;
export interface HasBalanceGte {
  address: string;
  required_balance: Balance;
  [k: string]: unknown;
}
export interface Coin {
  amount: Uint128;
  denom: string;
  [k: string]: unknown;
}
export interface Cw20CoinVerified {
  address: Addr;
  amount: Uint128;
  [k: string]: unknown;
}
export interface InstantiateMsg {
  [k: string]: unknown;
}
export type QueryConstructResponse = [boolean, number | null];
export type Rule = {
  has_balance_gte: HasBalanceGte;
} | {
  check_owner_of_nft: CheckOwnerOfNft;
} | {
  check_proposal_status: CheckProposalStatus;
} | {
  generic_query: GenericQuery;
};
export type ValueIndex = {
  key: string;
} | {
  index: number;
};
export type ValueOrdering = "unit_above" | "unit_above_equal" | "unit_below" | "unit_below_equal" | "equal";
export interface QueryConstruct {
  rules: Rule[];
  [k: string]: unknown;
}
export interface GenericQuery {
  contract_addr: string;
  gets: ValueIndex[];
  msg: Binary;
  ordering: ValueOrdering;
  value: Binary;
  [k: string]: unknown;
}
export type QueryMsg = {
  get_balance: {
    address: string;
    denom: string;
    [k: string]: unknown;
  };
} | {
  get_cw20_balance: {
    address: string;
    cw20_contract: string;
    [k: string]: unknown;
  };
} | {
  has_balance_gte: HasBalanceGte;
} | {
  check_owner_of_nft: CheckOwnerOfNft;
} | {
  check_passed_proposals: CheckPassedProposals;
} | {
  check_proposal_status: CheckProposalStatus;
} | {
  generic_query: GenericQuery;
} | {
  query_construct: QueryConstruct;
};
export interface CheckPassedProposals {
  dao_address: string;
  [k: string]: unknown;
}
export interface QueryMultiResponse {
  data: string[];
  [k: string]: unknown;
}
export type RuleResponse = [boolean, Binary | null];