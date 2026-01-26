#!/usr/bin/env bash
# $1 is tmux session name

START_TRANSPORT="npm run start-transport-robot;exec bash"
START_WAREHOUSE="npm run start-warehouse;exec bash"
START_ASSEMBLY="npm run start-assembly-robot;exec bash"

npm run build

bash ../split_and_run.sh $1 "$START_TRANSPORT" "$START_TRANSPORT" "$START_TRANSPORT" "$START_WAREHOUSE" "$START_ASSEMBLY"