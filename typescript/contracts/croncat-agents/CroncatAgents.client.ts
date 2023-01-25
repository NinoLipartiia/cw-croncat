/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.19.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { Coin, StdFee } from "@cosmjs/amino";
import { InstantiateMsg, ExecuteMsg, UpdateConfig, QueryMsg, Addr, Config, Nullable_AgentResponse, Uint128, Timestamp, Uint64, AgentStatus, AgentResponse, Nullable_GetAgentIdsResponse, GetAgentIdsResponse, Nullable_AgentTaskResponse, AgentTaskResponse } from "./CroncatAgents.types";
export interface CroncatAgentsReadOnlyInterface {
  contractAddress: string;
  getAgent: ({
    accountId,
    totalTasks
  }: {
    accountId: string;
    totalTasks: number;
  }) => Promise<NullableAgentResponse>;
  getAgentIds: ({
    skip,
    take
  }: {
    skip?: number;
    take?: number;
  }) => Promise<NullableGetAgentIdsResponse>;
  getAgentTasks: ({
    accountId,
    blockSlots,
    cronSlots
  }: {
    accountId: string;
    blockSlots?: number;
    cronSlots?: number;
  }) => Promise<NullableAgentTaskResponse>;
  config: () => Promise<Config>;
}
export class CroncatAgentsQueryClient implements CroncatAgentsReadOnlyInterface {
  client: CosmWasmClient;
  contractAddress: string;

  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client;
    this.contractAddress = contractAddress;
    this.getAgent = this.getAgent.bind(this);
    this.getAgentIds = this.getAgentIds.bind(this);
    this.getAgentTasks = this.getAgentTasks.bind(this);
    this.config = this.config.bind(this);
  }

  getAgent = async ({
    accountId,
    totalTasks
  }: {
    accountId: string;
    totalTasks: number;
  }): Promise<NullableAgentResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      get_agent: {
        account_id: accountId,
        total_tasks: totalTasks
      }
    });
  };
  getAgentIds = async ({
    skip,
    take
  }: {
    skip?: number;
    take?: number;
  }): Promise<NullableGetAgentIdsResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      get_agent_ids: {
        skip,
        take
      }
    });
  };
  getAgentTasks = async ({
    accountId,
    blockSlots,
    cronSlots
  }: {
    accountId: string;
    blockSlots?: number;
    cronSlots?: number;
  }): Promise<NullableAgentTaskResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      get_agent_tasks: {
        account_id: accountId,
        block_slots: blockSlots,
        cron_slots: cronSlots
      }
    });
  };
  config = async (): Promise<Config> => {
    return this.client.queryContractSmart(this.contractAddress, {
      config: {}
    });
  };
}
export interface CroncatAgentsInterface extends CroncatAgentsReadOnlyInterface {
  contractAddress: string;
  sender: string;
  registerAgent: ({
    payableAccountId
  }: {
    payableAccountId?: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  updateAgent: ({
    payableAccountId
  }: {
    payableAccountId: string;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  checkInAgent: (fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  unregisterAgent: ({
    fromBehind
  }: {
    fromBehind?: boolean;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  onTaskCreated: ({
    taskHash,
    totalTasks
  }: {
    taskHash: string;
    totalTasks: number;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
  updateConfig: ({
    config
  }: {
    config: UpdateConfig;
  }, fee?: number | StdFee | "auto", memo?: string, funds?: Coin[]) => Promise<ExecuteResult>;
}
export class CroncatAgentsClient extends CroncatAgentsQueryClient implements CroncatAgentsInterface {
  client: SigningCosmWasmClient;
  sender: string;
  contractAddress: string;

  constructor(client: SigningCosmWasmClient, sender: string, contractAddress: string) {
    super(client, contractAddress);
    this.client = client;
    this.sender = sender;
    this.contractAddress = contractAddress;
    this.registerAgent = this.registerAgent.bind(this);
    this.updateAgent = this.updateAgent.bind(this);
    this.checkInAgent = this.checkInAgent.bind(this);
    this.unregisterAgent = this.unregisterAgent.bind(this);
    this.onTaskCreated = this.onTaskCreated.bind(this);
    this.updateConfig = this.updateConfig.bind(this);
  }

  registerAgent = async ({
    payableAccountId
  }: {
    payableAccountId?: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      register_agent: {
        payable_account_id: payableAccountId
      }
    }, fee, memo, funds);
  };
  updateAgent = async ({
    payableAccountId
  }: {
    payableAccountId: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      update_agent: {
        payable_account_id: payableAccountId
      }
    }, fee, memo, funds);
  };
  checkInAgent = async (fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      check_in_agent: {}
    }, fee, memo, funds);
  };
  unregisterAgent = async ({
    fromBehind
  }: {
    fromBehind?: boolean;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      unregister_agent: {
        from_behind: fromBehind
      }
    }, fee, memo, funds);
  };
  onTaskCreated = async ({
    taskHash,
    totalTasks
  }: {
    taskHash: string;
    totalTasks: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      on_task_created: {
        task_hash: taskHash,
        total_tasks: totalTasks
      }
    }, fee, memo, funds);
  };
  updateConfig = async ({
    config
  }: {
    config: UpdateConfig;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      update_config: {
        config
      }
    }, fee, memo, funds);
  };
}