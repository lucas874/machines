#!/usr/bin/env bash
# $1 is tmux session name

version="KickTheTires"
START_TRANSPORT="npm run start-transport -- $version transport_log.txt; exec bash"
START_DOOR="npm run start-door -- $version door_log.txt; exec bash"
START_FORKLIFT="npm run start-forklift -- $version forklift_log.txt; exec bash"
START_ROBOT="npm run start-factory-robot -- $version robot_log.txt; exec bash"

npm run build

bash ../split_and_run.sh $1 "$START_TRANSPORT" "$START_DOOR" "$START_FORKLIFT" "$START_ROBOT"