#!/usr/bin/env bash

# $1 is tmux session name
# $2 is ax log
# $3 is build log

session_name=$1
ax_log=$2
build_log=$3

START_TRANSPORT="npm run start-transport-robot;exec bash"
START_WAREHOUSE="npm run start-warehouse;exec bash"
START_ASSEMBLY="npm run start-assembly-robot;exec bash"

npm run build >> $build_log 2>&1

bash ../split_and_run.sh $session_name $ax_log "$START_TRANSPORT" "$START_TRANSPORT" "$START_TRANSPORT" "$START_WAREHOUSE" "$START_ASSEMBLY"