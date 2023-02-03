/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.19.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { StdFee } from "@cosmjs/amino";
import { InstantiateMsg, GasPrice, ExecuteMsg, Uint128, Binary, Addr, UpdateConfig, Cw20Coin, Cw20ReceiveMsg, ManagerCreateTaskBalance, AmountForOneTask, Coin, Cw20CoinVerified, ManagerRemoveTask, WithdrawRewardsOnRemovalArgs, QueryMsg, Config, TaskBalanceResponse, TaskBalance, ArrayOfCw20CoinVerified } from "./CroncatManager.types";
export interface CroncatManagerReadOnlyInterface {
  contractAddress: string;
  config: () => Promise<Config>;
  treasuryBalance: () => Promise<Uint128>;
  usersBalances: ({
    fromIndex,
    limit,
    wallet
  }: {
    fromIndex?: number;
    limit?: number;
    wallet: string;
  }) => Promise<ArrayOfCw20CoinVerified>;
  taskBalance: ({
    taskHash
  }: {
    taskHash: string;
  }) => Promise<TaskBalanceResponse>;
  agentRewards: ({
    agentId
  }: {
    agentId: string;
  }) => Promise<Uint128>;
}
export class CroncatManagerQueryClient implements CroncatManagerReadOnlyInterface {
  client: CosmWasmClient;
  contractAddress: string;

  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client;
    this.contractAddress = contractAddress;
    this.config = this.config.bind(this);
    this.treasuryBalance = this.treasuryBalance.bind(this);
    this.usersBalances = this.usersBalances.bind(this);
    this.taskBalance = this.taskBalance.bind(this);
    this.agentRewards = this.agentRewards.bind(this);
  }

  config = async (): Promise<Config> => {
    return this.client.queryContractSmart(this.contractAddress, {
      config: {}
    });
  };
  treasuryBalance = async (): Promise<Uint128> => {
    return this.client.queryContractSmart(this.contractAddress, {
      treasury_balance: {}
    });
  };
  usersBalances = async ({
    fromIndex,
    limit,
    wallet
  }: {
    fromIndex?: number;
    limit?: number;
    wallet: string;
  }): Promise<ArrayOfCw20CoinVerified> => {
    return this.client.queryContractSmart(this.contractAddress, {
      users_balances: {
        from_index: fromIndex,
        limit,
        wallet
      }
    });
  };
  taskBalance = async ({
    taskHash
  }: {
    taskHash: string;
  }): Promise<TaskBalanceResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      task_balance: {
        task_hash: taskHash
      }
    });
  };
  agentRewards = async ({
    agentId
  }: {
    agentId: string;
  }): Promise<Uint128> => {
    return this.client.queryContractSmart(this.contractAddress, {
      agent_rewards: {
        agent_id: agentId
      }
    });
  };
}
export interface CroncatManagerInterface extends CroncatManagerReadOnlyInterface {
  contractAddress: string;
  sender: string;
  updateConfig: ({
    agentFee,
    croncatAgentsKey,
    croncatTasksKey,
    gasPrice,
    ownerAddr,
    paused,
    treasuryAddr,
    treasuryFee
  }: {
    agentFee?: number;
    croncatAgentsKey?: string[][];
    croncatTasksKey?: string[][];
    gasPrice?: GasPrice;
    ownerAddr?: string;
    paused?: boolean;
    treasuryAddr?: string;
    treasuryFee?: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  ownerWithdraw: (fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  proxyCall: ({
    taskHash
  }: {
    taskHash?: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  refillTaskBalance: ({
    taskHash
  }: {
    taskHash: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  refillTaskCw20Balance: ({
    cw20,
    taskHash
  }: {
    cw20: Cw20Coin;
    taskHash: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  receive: ({
    amount,
    msg,
    sender
  }: {
    amount: Uint128;
    msg: Binary;
    sender: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  userWithdraw: ({
    limit
  }: {
    limit?: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  tick: (fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  createTaskBalance: ({
    amountForOneTask,
    cw20,
    recurring,
    sender,
    taskHash
  }: {
    amountForOneTask: AmountForOneTask;
    cw20?: Cw20CoinVerified;
    recurring: boolean;
    sender: Addr;
    taskHash: number[];
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  removeTask: ({
    sender,
    taskHash
  }: {
    sender: Addr;
    taskHash: number[];
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  withdrawAgentRewards: (fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
}
export class CroncatManagerClient extends CroncatManagerQueryClient implements CroncatManagerInterface {
  client: SigningCosmWasmClient;
  sender: string;
  contractAddress: string;

  constructor(client: SigningCosmWasmClient, sender: string, contractAddress: string) {
    super(client, contractAddress);
    this.client = client;
    this.sender = sender;
    this.contractAddress = contractAddress;
    this.updateConfig = this.updateConfig.bind(this);
    this.ownerWithdraw = this.ownerWithdraw.bind(this);
    this.proxyCall = this.proxyCall.bind(this);
    this.refillTaskBalance = this.refillTaskBalance.bind(this);
    this.refillTaskCw20Balance = this.refillTaskCw20Balance.bind(this);
    this.receive = this.receive.bind(this);
    this.userWithdraw = this.userWithdraw.bind(this);
    this.tick = this.tick.bind(this);
    this.createTaskBalance = this.createTaskBalance.bind(this);
    this.removeTask = this.removeTask.bind(this);
    this.withdrawAgentRewards = this.withdrawAgentRewards.bind(this);
  }

  updateConfig = async ({
    agentFee,
    croncatAgentsKey,
    croncatTasksKey,
    gasPrice,
    ownerAddr,
    paused,
    treasuryAddr,
    treasuryFee
  }: {
    agentFee?: number;
    croncatAgentsKey?: string[][];
    croncatTasksKey?: string[][];
    gasPrice?: GasPrice;
    ownerAddr?: string;
    paused?: boolean;
    treasuryAddr?: string;
    treasuryFee?: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      update_config: {
        agent_fee: agentFee,
        croncat_agents_key: croncatAgentsKey,
        croncat_tasks_key: croncatTasksKey,
        gas_price: gasPrice,
        owner_addr: ownerAddr,
        paused,
        treasury_addr: treasuryAddr,
        treasury_fee: treasuryFee
      }
    }, fee, memo, funds);
  };
  ownerWithdraw = async (fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      owner_withdraw: {}
    }, fee, memo, funds);
  };
  proxyCall = async ({
    taskHash
  }: {
    taskHash?: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      proxy_call: {
        task_hash: taskHash
      }
    }, fee, memo, funds);
  };
  refillTaskBalance = async ({
    taskHash
  }: {
    taskHash: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      refill_task_balance: {
        task_hash: taskHash
      }
    }, fee, memo, funds);
  };
  refillTaskCw20Balance = async ({
    cw20,
    taskHash
  }: {
    cw20: Cw20Coin;
    taskHash: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      refill_task_cw20_balance: {
        cw20,
        task_hash: taskHash
      }
    }, fee, memo, funds);
  };
  receive = async ({
    amount,
    msg,
    sender
  }: {
    amount: Uint128;
    msg: Binary;
    sender: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      receive: {
        amount,
        msg,
        sender
      }
    }, fee, memo, funds);
  };
  userWithdraw = async ({
    limit
  }: {
    limit?: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      user_withdraw: {
        limit
      }
    }, fee, memo, funds);
  };
  tick = async (fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      tick: {}
    }, fee, memo, funds);
  };
  createTaskBalance = async ({
    amountForOneTask,
    cw20,
    recurring,
    sender,
    taskHash
  }: {
    amountForOneTask: AmountForOneTask;
    cw20?: Cw20CoinVerified;
    recurring: boolean;
    sender: Addr;
    taskHash: number[];
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      create_task_balance: {
        amount_for_one_task: amountForOneTask,
        cw20,
        recurring,
        sender,
        task_hash: taskHash
      }
    }, fee, memo, funds);
  };
  removeTask = async ({
    sender,
    taskHash
  }: {
    sender: Addr;
    taskHash: number[];
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      remove_task: {
        sender,
        task_hash: taskHash
      }
    }, fee, memo, funds);
  };
  withdrawAgentRewards = async (fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      withdraw_agent_rewards: {}
    }, fee, memo, funds);
  };
}