#!/bin/bash
set -e

cargo wasm
# In case of M1 MacBook use rust-optimizer-arm64 instead of rust-optimizer
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer-arm64:0.12.6

NODE="--node https://rpc.uni.juno.deuslabs.fi:443"
TXFLAG="--node https://rpc.uni.juno.deuslabs.fi:443 --chain-id uni-3 --gas-prices 0.025ujunox --gas auto --gas-adjustment 1.3 --broadcast-mode block"

# Make sure OWNER and USER wallets have enough JUNOX
OWNER="$1"
AGENT="$2"
USER="$3"

# In case of M1 MacBook replace cw_croncat.wasm with cw_croncat-aarch64.wasm 
RES=$(junod tx wasm store artifacts/cw_croncat-aarch64.wasm --from $OWNER $TXFLAG -y --output json -b block)
CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')

# Instantiate
INIT='{"denom":"ujunox"}'
junod tx wasm instantiate $CODE_ID "$INIT" --from $OWNER --label "croncat" $TXFLAG -y --no-admin
CONTRACT=$(junod query wasm list-contract-by-code $CODE_ID $NODE --output json | jq -r '.contracts[-1]')

# Now we can register an agent, create tasks and execute a task
# Register an agent
REGISTER_AGENT='{"register_agent":{}}'
junod tx wasm execute $CONTRACT "$REGISTER_AGENT" --from $AGENT $TXFLAG -y

# Create tasks for staking, sending and burning tokens
STAKE='{"create_task":{"task":{"interval":"Once","boundary":null,"stop_on_fail":false,"actions":[{"msg":{"staking":{"delegate":{"validator":"juno14vhcdsyf83ngsrrqc92kmw8q9xakqjm0ff2dpn","amount":{"denom":"ujunox","amount":"1000000"}}}},"gas_limit":150000}],"rules":null}}}'
junod tx wasm execute $CONTRACT "$STAKE" --amount 1000000ujunox --from $USER $TXFLAG -y

BANK_SEND='{"create_task":{"task":{"interval":"Once","boundary":null,"stop_on_fail":false,"actions":[{"msg":{"bank":{"send":{"to_address":"juno1e48ptufsh6yplls6vmy4827uxnxlmpdul0nqkg","amount":[{"denom":"ujunox","amount":10000}]}}},"gas_limit":150000}],"rules":null}}}'
junod tx wasm execute $CONTRACT "$BANK_SEND" --amount 1000000ujunox --from $USER $TXFLAG -y

BANK_BURN='{"create_task":{"task":{"interval":"Once","boundary":null,"stop_on_fail":false,"actions":[{"msg":{"bank":{"burn":{"amount":[{"denom":"ujunox","amount":10000}]}}},"gas_limit":150000}],"rules":null}}}'
junod tx wasm execute $CONTRACT "$BANK_BURN" --amount 1000000ujunox --from $USER $TXFLAG -y

# proxy_call
sleep 10      # is needed to make sure this call in the next block 
PROXY_CALL='{"proxy_call":{}}'
junod tx wasm execute $CONTRACT "$PROXY_CALL" --from $AGENT $TXFLAG -y
junod tx wasm execute $CONTRACT "$PROXY_CALL" --from $AGENT $TXFLAG -y
junod tx wasm execute $CONTRACT "$PROXY_CALL" --from $AGENT $TXFLAG -y
