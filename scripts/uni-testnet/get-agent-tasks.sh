#!/bin/bash

SH_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
SH_DIR="$(cd -P "$(dirname "${SH_PATH}")";pwd)"
. $SH_DIR/base/init-vars.sh

if [ -z "$1" ]
then
    echo "Must provide contract address"
    exit 1
elif [ -z "$2" ]
then
    echo "Must provide agent address"
    exit 1
else
    CONTRACT="$1"
    AGENT="$2"
fi

GET_AGENT_TASKS='{"get_agent_tasks":{"account_id":"'$(junod keys show $AGENT -a)'"}}'
junod query wasm contract-state smart $CONTRACT "$GET_AGENT_TASKS" $NODE
