"use strict";
/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.14.2.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/
Object.defineProperty(exports, "__esModule", { value: true });
exports.useCwCroncatUpdateSettingsMutation = exports.useCwCroncatMoveBalancesMutation = exports.useCwCroncatRegisterAgentMutation = exports.useCwCroncatUpdateAgentMutation = exports.useCwCroncatCheckInAgentMutation = exports.useCwCroncatUnregisterAgentMutation = exports.useCwCroncatWithdrawRewardMutation = exports.useCwCroncatCreateTaskMutation = exports.useCwCroncatRemoveTaskMutation = exports.useCwCroncatRefillTaskBalanceMutation = exports.useCwCroncatRefillTaskCw20BalanceMutation = exports.useCwCroncatProxyCallMutation = exports.useCwCroncatReceiveMutation = exports.useCwCroncatWithdrawWalletBalanceMutation = exports.useCwCroncatGetConfigQuery = exports.useCwCroncatGetBalancesQuery = exports.useCwCroncatGetAgentQuery = exports.useCwCroncatGetAgentIdsQuery = exports.useCwCroncatGetAgentTasksQuery = exports.useCwCroncatGetTasksQuery = exports.useCwCroncatGetTasksWithRulesQuery = exports.useCwCroncatGetTasksByOwnerQuery = exports.useCwCroncatGetTaskQuery = exports.useCwCroncatGetTaskHashQuery = exports.useCwCroncatValidateIntervalQuery = exports.useCwCroncatGetSlotHashesQuery = exports.useCwCroncatGetSlotIdsQuery = exports.useCwCroncatGetWalletBalancesQuery = exports.useCwCroncatGetStateQuery = exports.cwCroncatQueryKeys = void 0;
const react_query_1 = require("@tanstack/react-query");
exports.cwCroncatQueryKeys = {
    contract: [{
            contract: "cwCroncat"
        }],
    address: (contractAddress) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.contract[0]), { address: contractAddress })],
    getConfig: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_config", args })],
    getBalances: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_balances", args })],
    getAgent: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_agent", args })],
    getAgentIds: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_agent_ids", args })],
    getAgentTasks: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_agent_tasks", args })],
    getTasks: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_tasks", args })],
    getTasksWithRules: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_tasks_with_rules", args })],
    getTasksByOwner: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_tasks_by_owner", args })],
    getTask: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_task", args })],
    getTaskHash: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_task_hash", args })],
    validateInterval: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "validate_interval", args })],
    getSlotHashes: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_slot_hashes", args })],
    getSlotIds: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_slot_ids", args })],
    getWalletBalances: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_wallet_balances", args })],
    getState: (contractAddress, args) => [Object.assign(Object.assign({}, exports.cwCroncatQueryKeys.address(contractAddress)[0]), { method: "get_state", args })]
};
function useCwCroncatGetStateQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getState(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getState({
        fromIndex: args.fromIndex,
        limit: args.limit
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetStateQuery = useCwCroncatGetStateQuery;
function useCwCroncatGetWalletBalancesQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getWalletBalances(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getWalletBalances({
        wallet: args.wallet
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetWalletBalancesQuery = useCwCroncatGetWalletBalancesQuery;
function useCwCroncatGetSlotIdsQuery({ client, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getSlotIds(client === null || client === void 0 ? void 0 : client.contractAddress), () => client ? client.getSlotIds() : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetSlotIdsQuery = useCwCroncatGetSlotIdsQuery;
function useCwCroncatGetSlotHashesQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getSlotHashes(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getSlotHashes({
        slot: args.slot
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetSlotHashesQuery = useCwCroncatGetSlotHashesQuery;
function useCwCroncatValidateIntervalQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.validateInterval(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.validateInterval({
        interval: args.interval
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatValidateIntervalQuery = useCwCroncatValidateIntervalQuery;
function useCwCroncatGetTaskHashQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getTaskHash(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getTaskHash({
        task: args.task
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetTaskHashQuery = useCwCroncatGetTaskHashQuery;
function useCwCroncatGetTaskQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getTask(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getTask({
        taskHash: args.taskHash
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetTaskQuery = useCwCroncatGetTaskQuery;
function useCwCroncatGetTasksByOwnerQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getTasksByOwner(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getTasksByOwner({
        ownerId: args.ownerId
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetTasksByOwnerQuery = useCwCroncatGetTasksByOwnerQuery;
function useCwCroncatGetTasksWithRulesQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getTasksWithRules(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getTasksWithRules({
        fromIndex: args.fromIndex,
        limit: args.limit
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetTasksWithRulesQuery = useCwCroncatGetTasksWithRulesQuery;
function useCwCroncatGetTasksQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getTasks(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getTasks({
        fromIndex: args.fromIndex,
        limit: args.limit
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetTasksQuery = useCwCroncatGetTasksQuery;
function useCwCroncatGetAgentTasksQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getAgentTasks(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getAgentTasks({
        accountId: args.accountId
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetAgentTasksQuery = useCwCroncatGetAgentTasksQuery;
function useCwCroncatGetAgentIdsQuery({ client, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getAgentIds(client === null || client === void 0 ? void 0 : client.contractAddress), () => client ? client.getAgentIds() : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetAgentIdsQuery = useCwCroncatGetAgentIdsQuery;
function useCwCroncatGetAgentQuery({ client, args, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getAgent(client === null || client === void 0 ? void 0 : client.contractAddress, args), () => client ? client.getAgent({
        accountId: args.accountId
    }) : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetAgentQuery = useCwCroncatGetAgentQuery;
function useCwCroncatGetBalancesQuery({ client, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getBalances(client === null || client === void 0 ? void 0 : client.contractAddress), () => client ? client.getBalances() : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetBalancesQuery = useCwCroncatGetBalancesQuery;
function useCwCroncatGetConfigQuery({ client, options }) {
    return (0, react_query_1.useQuery)(exports.cwCroncatQueryKeys.getConfig(client === null || client === void 0 ? void 0 : client.contractAddress), () => client ? client.getConfig() : Promise.reject(new Error("Invalid client")), Object.assign(Object.assign({}, options), { enabled: !!client && ((options === null || options === void 0 ? void 0 : options.enabled) != undefined ? options.enabled : true) }));
}
exports.useCwCroncatGetConfigQuery = useCwCroncatGetConfigQuery;
function useCwCroncatWithdrawWalletBalanceMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.withdrawWalletBalance(msg, fee, memo, funds), options);
}
exports.useCwCroncatWithdrawWalletBalanceMutation = useCwCroncatWithdrawWalletBalanceMutation;
function useCwCroncatReceiveMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.receive(msg, fee, memo, funds), options);
}
exports.useCwCroncatReceiveMutation = useCwCroncatReceiveMutation;
function useCwCroncatProxyCallMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.proxyCall(msg, fee, memo, funds), options);
}
exports.useCwCroncatProxyCallMutation = useCwCroncatProxyCallMutation;
function useCwCroncatRefillTaskCw20BalanceMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.refillTaskCw20Balance(msg, fee, memo, funds), options);
}
exports.useCwCroncatRefillTaskCw20BalanceMutation = useCwCroncatRefillTaskCw20BalanceMutation;
function useCwCroncatRefillTaskBalanceMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.refillTaskBalance(msg, fee, memo, funds), options);
}
exports.useCwCroncatRefillTaskBalanceMutation = useCwCroncatRefillTaskBalanceMutation;
function useCwCroncatRemoveTaskMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.removeTask(msg, fee, memo, funds), options);
}
exports.useCwCroncatRemoveTaskMutation = useCwCroncatRemoveTaskMutation;
function useCwCroncatCreateTaskMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.createTask(msg, fee, memo, funds), options);
}
exports.useCwCroncatCreateTaskMutation = useCwCroncatCreateTaskMutation;
function useCwCroncatWithdrawRewardMutation(options) {
    return (0, react_query_1.useMutation)(({ client, args: { fee, memo, funds } = {} }) => client.withdrawReward(fee, memo, funds), options);
}
exports.useCwCroncatWithdrawRewardMutation = useCwCroncatWithdrawRewardMutation;
function useCwCroncatUnregisterAgentMutation(options) {
    return (0, react_query_1.useMutation)(({ client, args: { fee, memo, funds } = {} }) => client.unregisterAgent(fee, memo, funds), options);
}
exports.useCwCroncatUnregisterAgentMutation = useCwCroncatUnregisterAgentMutation;
function useCwCroncatCheckInAgentMutation(options) {
    return (0, react_query_1.useMutation)(({ client, args: { fee, memo, funds } = {} }) => client.checkInAgent(fee, memo, funds), options);
}
exports.useCwCroncatCheckInAgentMutation = useCwCroncatCheckInAgentMutation;
function useCwCroncatUpdateAgentMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.updateAgent(msg, fee, memo, funds), options);
}
exports.useCwCroncatUpdateAgentMutation = useCwCroncatUpdateAgentMutation;
function useCwCroncatRegisterAgentMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.registerAgent(msg, fee, memo, funds), options);
}
exports.useCwCroncatRegisterAgentMutation = useCwCroncatRegisterAgentMutation;
function useCwCroncatMoveBalancesMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.moveBalances(msg, fee, memo, funds), options);
}
exports.useCwCroncatMoveBalancesMutation = useCwCroncatMoveBalancesMutation;
function useCwCroncatUpdateSettingsMutation(options) {
    return (0, react_query_1.useMutation)(({ client, msg, args: { fee, memo, funds } = {} }) => client.updateSettings(msg, fee, memo, funds), options);
}
exports.useCwCroncatUpdateSettingsMutation = useCwCroncatUpdateSettingsMutation;