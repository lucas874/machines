#!/usr/bin/env bash

# $1 is tmux session name
# $2 is ax log
# $3 is build log
# $4 is log file for transport machine
# $5 is log file for door machine
# $6 is log file for forklift machine
# $7 is log file for assembly robot machine

session_name=$1
ax_log=$2
build_log=$3
transport_log=$4
door_log=$5
forklift_log=$6
robot_log=$7

version="KickTheTires"
START_TRANSPORT="npm run start-transport -- $version ${transport_log}; exec bash"
START_DOOR="npm run start-door -- $version ${door_log}; exec bash"
START_FORKLIFT="npm run start-forklift -- $version ${forklift_log}; exec bash"
START_ROBOT="npm run start-factory-robot -- $version ${robot_log}; exec bash"

npm run build >> $build_log 2>&1

bash ../split_and_run.sh $session_name $ax_log "$START_TRANSPORT" "$START_DOOR" "$START_FORKLIFT" "$START_ROBOT"