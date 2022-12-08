/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.19.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { StdFee } from "@cosmjs/amino";
import { Binary, CheckOwnerOfNftResponse, CheckOwnerOfNft, CheckProposalStatusResponse, Status, CheckProposalStatus, CroncatQuery, Balance, Uint128, NativeBalance, Addr, ValueOrdering, ValueIndex, PathToValue, SmartQueries, HasBalanceGte, Coin, Cw20CoinVerified, GenericQuery, SmartQueryHead, SmartQuery, ExecuteMsg, GenericQueryResponse, GetBalanceResponse, GetCw20BalanceResponse, HasBalanceGteResponse, InstantiateMsg, QueryConstructResponse, QueryConstruct, QueryMsg, QueryMultiResponse, RuleResponse, SmartQueryResponse } from "./CwRulesCore.types";
export interface CwRulesCoreReadOnlyInterface {
  contractAddress: string;
  getBalance: ({
    address,
    denom
  }: {
    address: string;
    denom: string;
  }) => Promise<GetBalanceResponse>;
  getCw20Balance: ({
    address,
    cw20Contract
  }: {
    address: string;
    cw20Contract: string;
  }) => Promise<GetCw20BalanceResponse>;
  hasBalanceGte: ({
    address,
    requiredBalance
  }: {
    address: string;
    requiredBalance: Balance;
  }) => Promise<HasBalanceGteResponse>;
  checkOwnerOfNft: ({
    address,
    nftAddress,
    tokenId
  }: {
    address: string;
    nftAddress: string;
    tokenId: string;
  }) => Promise<CheckOwnerOfNftResponse>;
  checkProposalStatus: ({
    daoAddress,
    proposalId,
    status
  }: {
    daoAddress: string;
    proposalId: number;
    status: Status;
  }) => Promise<CheckProposalStatusResponse>;
  genericQuery: ({
    contractAddr,
    msg,
    ordering,
    pathToValue,
    value
  }: {
    contractAddr: string;
    msg: Binary;
    ordering: ValueOrdering;
    pathToValue: PathToValue;
    value: Binary;
  }) => Promise<GenericQueryResponse>;
  queryConstruct: ({
    queries
  }: {
    queries: CroncatQuery[];
  }) => Promise<QueryConstructResponse>;
  smartQuery: ({
    contractAddr,
    msg,
    ordering,
    pathToQueryValue,
    queries,
    value
  }: {
    contractAddr: string;
    msg: Binary;
    ordering: ValueOrdering;
    pathToQueryValue: PathToValue;
    queries: SmartQueries;
    value: Binary;
  }) => Promise<SmartQueryResponse>;
}
export class CwRulesCoreQueryClient implements CwRulesCoreReadOnlyInterface {
  client: CosmWasmClient;
  contractAddress: string;

  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client;
    this.contractAddress = contractAddress;
    this.getBalance = this.getBalance.bind(this);
    this.getCw20Balance = this.getCw20Balance.bind(this);
    this.hasBalanceGte = this.hasBalanceGte.bind(this);
    this.checkOwnerOfNft = this.checkOwnerOfNft.bind(this);
    this.checkProposalStatus = this.checkProposalStatus.bind(this);
    this.genericQuery = this.genericQuery.bind(this);
    this.queryConstruct = this.queryConstruct.bind(this);
    this.smartQuery = this.smartQuery.bind(this);
  }

  getBalance = async ({
    address,
    denom
  }: {
    address: string;
    denom: string;
  }): Promise<GetBalanceResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      get_balance: {
        address,
        denom
      }
    });
  };
  getCw20Balance = async ({
    address,
    cw20Contract
  }: {
    address: string;
    cw20Contract: string;
  }): Promise<GetCw20BalanceResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      get_cw20_balance: {
        address,
        cw20_contract: cw20Contract
      }
    });
  };
  hasBalanceGte = async ({
    address,
    requiredBalance
  }: {
    address: string;
    requiredBalance: Balance;
  }): Promise<HasBalanceGteResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      has_balance_gte: {
        address,
        required_balance: requiredBalance
      }
    });
  };
  checkOwnerOfNft = async ({
    address,
    nftAddress,
    tokenId
  }: {
    address: string;
    nftAddress: string;
    tokenId: string;
  }): Promise<CheckOwnerOfNftResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      check_owner_of_nft: {
        address,
        nft_address: nftAddress,
        token_id: tokenId
      }
    });
  };
  checkProposalStatus = async ({
    daoAddress,
    proposalId,
    status
  }: {
    daoAddress: string;
    proposalId: number;
    status: Status;
  }): Promise<CheckProposalStatusResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      check_proposal_status: {
        dao_address: daoAddress,
        proposal_id: proposalId,
        status
      }
    });
  };
  genericQuery = async ({
    contractAddr,
    msg,
    ordering,
    pathToValue,
    value
  }: {
    contractAddr: string;
    msg: Binary;
    ordering: ValueOrdering;
    pathToValue: PathToValue;
    value: Binary;
  }): Promise<GenericQueryResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      generic_query: {
        contract_addr: contractAddr,
        msg,
        ordering,
        path_to_value: pathToValue,
        value
      }
    });
  };
  queryConstruct = async ({
    queries
  }: {
    queries: CroncatQuery[];
  }): Promise<QueryConstructResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      query_construct: {
        queries
      }
    });
  };
  smartQuery = async ({
    contractAddr,
    msg,
    ordering,
    pathToQueryValue,
    queries,
    value
  }: {
    contractAddr: string;
    msg: Binary;
    ordering: ValueOrdering;
    pathToQueryValue: PathToValue;
    queries: SmartQueries;
    value: Binary;
  }): Promise<SmartQueryResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      smart_query: {
        contract_addr: contractAddr,
        msg,
        ordering,
        path_to_query_value: pathToQueryValue,
        queries,
        value
      }
    });
  };
}
export interface CwRulesCoreInterface extends CwRulesCoreReadOnlyInterface {
  contractAddress: string;
  sender: string;
  queryResult: (fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
}
export class CwRulesCoreClient extends CwRulesCoreQueryClient implements CwRulesCoreInterface {
  client: SigningCosmWasmClient;
  sender: string;
  contractAddress: string;

  constructor(client: SigningCosmWasmClient, sender: string, contractAddress: string) {
    super(client, contractAddress);
    this.client = client;
    this.sender = sender;
    this.contractAddress = contractAddress;
    this.queryResult = this.queryResult.bind(this);
  }

  queryResult = async (fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      query_result: {}
    }, fee, memo, funds);
  };
}