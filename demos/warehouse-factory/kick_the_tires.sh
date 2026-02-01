#!/usr/bin/env bash

# $1 is tmux session name
# $2 is log file for transport machine
# $3 is log file for door machine
# $4 is log file for forklift machine
# $5 is log file for assembly robot machine

version="KickTheTires"
transport_log=$2
door_log=$3
forklift_log=$4
robot_log=$5
START_TRANSPORT="npm run start-transport -- $version ${transport_log}; exec bash"
START_DOOR="npm run start-door -- $version ${door_log}; exec bash"
START_FORKLIFT="npm run start-forklift -- $version ${forklift_log}; exec bash"
START_ROBOT="npm run start-factory-robot -- $version ${robot_log}; exec bash"

npm run build

bash ../split_and_run.sh $1 "$START_TRANSPORT" "$START_DOOR" "$START_FORKLIFT" "$START_ROBOT"