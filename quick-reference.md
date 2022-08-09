# Build the contract
Building:
```bash
sh build.sh
```
Optimizing the binary size:
```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.6
```
> In case of M1 MacBook use `rust-optimizer-arm64` instead of `rust-optimizer`

# Setup the variables
Set up these variables, so that you don't have to type in node, chain id and gas-price details every time you execute commands.
```bash
NODE="--node https://rpc.uni.juno.deuslabs.fi:443"
TXFLAG="--node https://rpc.uni.juno.deuslabs.fi:443 --chain-id uni-3 --gas-prices 0.025ujunox --gas auto --gas-adjustment 1.3 --broadcast-mode block"
```
Specify your addresses, which will be responsible for owner of the contract, agent and the user, who creates tasks:
```bash
OWNER=your-owner-address
USER=creator-of-the-tasks-address
AGENT=your-agent-address
```
Note, that `OWNER`'s balance must be enough for storing a wasm file.
`USER` will spend some tokens creating tasks, `AGENT` only pays gas fees.

Alternatively you can create new wallets and request some JUNOX from the faucet:
```bash
OWNER=owner$RANDOM
AGENT=agent$RANDOM
USER=user$RANDOM

junod keys add $OWNER
junod keys add $AGENT
junod keys add $USER

JSON=$(jq -n --arg addr $(junod keys show -a $OWNER) '{ denom:"ujunox","address":$addr}') && \
  curl -X POST --header "Content-Type: application/json" --data "$JSON" https://faucet.uni.juno.deuslabs.fi/credit && echo
JSON=$(jq -n --arg addr $(junod keys show -a $AGENT) '{ denom:"ujunox","address":$addr}') && \
  curl -X POST --header "Content-Type: application/json" --data "$JSON" https://faucet.uni.juno.deuslabs.fi/credit && echo
JSON=$(jq -n --arg addr $(junod keys show -a $USER) '{ denom:"ujunox","address":$addr}') && \
  curl -X POST --header "Content-Type: application/json" --data "$JSON" https://faucet.uni.juno.deuslabs.fi/credit && echo
```

# Store the code
Store the code to the uni-3 testnet:
```bash
RES=$(junod tx wasm store artifacts/cw_croncat.wasm --from $OWNER $TXFLAG -y --output json -b block)
CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')
```
> In case of M1 MacBook replace `cw_croncat.wasm` with `cw_croncat-aarch64.wasm` 
# Instantiate the contract
```bash
INIT='{"denom":"ujunox"}'
junod tx wasm instantiate $CODE_ID "$INIT" --from $OWNER --label "croncat" $TXFLAG -y --no-admin
CONTRACT=$(junod query wasm list-contract-by-code $CODE_ID $NODE --output json | jq -r '.contracts[-1]')
```
# Interacting with croncat

## Tasks
Here `USER` creates two tasks:
```bash
STAKE='{"create_task":{"task":{"interval":"Immediate","boundary":null,"stop_on_fail":false,"actions":[{"msg":{"staking":{"delegate":{"validator":"juno14vhcdsyf83ngsrrqc92kmw8q9xakqjm0ff2dpn","amount":{"denom":"ujunox","amount":"10000"}}}},"gas_limit":150000}],"rules":null}}}'
junod tx wasm execute $CONTRACT "$STAKE" --amount 500000ujunox --from $USER $TXFLAG -y

STAKE2='{"create_task":{"task":{"interval":"Immediate","boundary":null,"stop_on_fail":false,"actions":[{"msg":{"staking":{"delegate":{"validator":"juno14vhcdsyf83ngsrrqc92kmw8q9xakqjm0ff2dpn","amount":{"denom":"ujunox","amount":"20000"}}}},"gas_limit":150000}],"rules":null}}}'
junod tx wasm execute $CONTRACT "$STAKE2" --amount 500000ujunox --from $USER $TXFLAG -y
```
`USER` can refill the second task:
```bash
REFILL_TASK_BALANCE='{"refill_task_balance":{"task_hash":"a34be29ee9bd34c3239a10d00ef9f675ff8f3fab241707dcb688d2fdd2cf0e75"}}'
junod tx wasm execute $CONTRACT "$REFILL_TASK_BALANCE" --amount 200000ujunox --from $USER $TXFLAG -y
```
He also may remove the task:
```bash
REMOVE_TASK='{"remove_task":{"task_hash":"a34be29ee9bd34c3239a10d00ef9f675ff8f3fab241707dcb688d2fdd2cf0e75"}}'
junod tx wasm execute $CONTRACT "$REMOVE_TASK" --from $USER $TXFLAG -y
```

## Agents
`AGENT` registers as agent. Since he is the first agent, he will be automatically put in the list of active agents.
```bash
REGISTER_AGENT='{"register_agent":{}}'
junod tx wasm execute $CONTRACT "$REGISTER_AGENT" --from $AGENT $TXFLAG -y
```
The agent can change the payment account:
```bash
UPDATE_AGENT='{"update_agent":{"payable_account_id":"'$(junod keys show $USER -a)'"}}'
junod tx wasm execute $CONTRACT "$UPDATE_AGENT" --from $AGENT $TXFLAG -y
```
`AGENT` should execute tasks by calling `proxy_call`
```bash
PROXY_CALL='{"proxy_call":{}}'
junod tx wasm execute $CONTRACT "$PROXY_CALL" --from $AGENT $TXFLAG -y
```
After executing tasks he can withdraw the tokens that he earned:
```bash
WITHDRAW_REWARD='{"withdraw_reward":{}}'
junod tx wasm execute $CONTRACT "$WITHDRAW_REWARD" --from $AGENT $TXFLAG -y
```
To withdraw the reward and unregister:
```bash
UNREGISTER_AGENT='{"unregister_agent":{}}'
junod tx wasm execute $CONTRACT "$UNREGISTER_AGENT" --from $AGENT $TXFLAG -y
```
When the second agent tries to register, he is put in pending list. 
If there are enough tasks, one can accept nomination and become an agent.
Otherwise the user needs to wait when more tasks are added and accepts nomination.
```bash
REGISTER_AGENT='{"register_agent":{}}'
junod tx wasm execute $CONTRACT "$REGISTER_AGENT" --from $OWNER $TXFLAG -y
junod tx wasm execute $CONTRACT "$REGISTER_AGENT" --from $USER $TXFLAG -y

STAKE3='{"create_task":{"task":{"interval":"Immediate","boundary":null,"stop_on_fail":false,"actions":[{"msg":{"staking":{"delegate":{"validator":"juno14vhcdsyf83ngsrrqc92kmw8q9xakqjm0ff2dpn","amount":{"denom":"ujunox","amount":"300000"}}}},"gas_limit":150000}],"rules":null}}}'
junod tx wasm execute $CONTRACT "$STAKE3" --amount 500000ujunox --from $USER $TXFLAG -y

CHECK_IN_AGENT='{"check_in_agent":{}}'
junod tx wasm execute $CONTRACT "$CHECK_IN_AGENT" --from $USER $TXFLAG -y
```

## Owner
Only `OWNER` can update settings of the contract.
For example, here we pause it for creating and executing tasks, registering agents:
```bash
UPDATE_SETTINGS='{"update_settings":{"paused":true}}'
junod tx wasm execute $CONTRACT "$UPDATE_SETTINGS" --from $OWNER $TXFLAG -y
```
`OWNER` may move balances from contract to his address.
```bash
MOVE_BALANCES='{"move_balances":{"balances":[],"account_id":"'$(junod keys show $OWNER -a)'"}}'
junod tx wasm execute $CONTRACT "$MOVE_BALANCES" --from $OWNER $TXFLAG -y
```
## Query
To get the config:
```bash
GET_CONFIG='{"get_config":{}}'
junod query wasm contract-state smart $CONTRACT "$GET_CONFIG" $NODE
```
To get balances of the contract address:
```bash
GET_BALANCES='{"get_balances":{}}'
junod query wasm contract-state smart $CONTRACT "$GET_BALANCES" $NODE
```
To get agent details:
```bash
GET_AGENT='{"get_agent":{"account_id":"'$(junod keys show $USER -a)'"}}'
junod query wasm contract-state smart $CONTRACT "$GET_AGENT" $NODE
```
To get addresses of active and pending agents:
```bash
GET_AGENT_IDS='{"get_agent_ids":{}}'
junod query wasm contract-state smart $CONTRACT "$GET_AGENT_IDS" $NODE
```
To get info about tasks for the agent:
```bash
GET_AGENT_TASKS='{"get_agent_tasks":{"account_id":"'$(junod keys show $USER -a)'"}}'
junod query wasm contract-state smart $CONTRACT "$GET_AGENT_TASKS" $NODE
```
To get details of all tasks:
```bash
GET_TASKS='{"get_tasks":{}}'
junod query wasm contract-state smart $CONTRACT "$GET_TASKS" $NODE
```
To see tasks created by the specific user:
```bash
GET_TASKS_BY_OWNER='{"get_tasks_by_owner":{"owner_id":"'$(junod keys show $USER -a)'"}}'
junod query wasm contract-state smart $CONTRACT "$GET_TASKS_BY_OWNER" $NODE
```
To get a task by the hash:
```bash
GET_TASK='{"get_task":{"task_hash":"2cfc83749ca11d1ea8461cf919a3eee7f0e7fc5246ab0694add1b54473d46b03"}}'
junod query wasm contract-state smart $CONTRACT "$GET_TASK" $NODE
```
To get a hash of the task:
```bash
GET_TASK_HASH='{"get_task_hash":{"task":{"owner_id":"'$(junod keys show $OWNER -a)'","interval":"Immediate","boundary":{"start":null,"end":null},"stop_on_fail":false,"total_deposit":[{"denom":"ujunox","amount":"500000"}],"actions":[{"msg":{"staking":{"delegate":{"validator":"juno14vhcdsyf83ngsrrqc92kmw8q9xakqjm0ff2dpn","amount":{"denom":"ujunox","amount":"300000"}}}},"gas_limit":150000}],"rules":null}}}'
junod query wasm contract-state smart $CONTRACT "$GET_TASK_HASH" $NODE
```
Check if the interval is valid:
```bash
VALIDATE_INTERVAL='{"validate_interval":{"interval":"Once"}}'
junod query wasm contract-state smart $CONTRACT "$VALIDATE_INTERVAL" $NODE
```
To get the next executable set of tasks hashes:

```bash
GET_SLOT_HASHES='{"get_slot_hashes":{}}'
junod query wasm contract-state smart $CONTRACT "$GET_SLOT_HASHES" $NODE

GET_SLOT_HASHES_SLOT='{"get_slot_hashes":{"slot":800000}}'
junod query wasm contract-state smart $CONTRACT "$GET_SLOT_HASHES_SLOT" $NODE
```
To gets list of active slot ids, for both time and block slots:
```bash
GET_SLOT_IDS='{"get_slot_ids":{}}'
junod query wasm contract-state smart $CONTRACT "$GET_SLOT_IDS" $NODE
```