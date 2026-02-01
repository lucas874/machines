#!/usr/bin/env bash

# $1 is tmux session name
# $2 is ax log
# $3 is build log

session_name=$1
ax_log=$2
build_log=$3

version="WarehouseFactoryQuality"
START_TRANSPORT="npm run start-transport -- $version; exec bash"
START_DOOR="npm run start-door -- $version; exec bash"
START_FORKLIFT="npm run start-forklift -- $version; exec bash"
START_ROBOT="npm run start-factory-robot -- $version; exec bash"
START_QUALITY_CONTROL="npm run start-quality-control -- $version; exec bash"

npm run build >> $build_log 2>&1

bash ../split_and_run.sh $session_name $ax_log "$START_TRANSPORT" "$START_DOOR" "$START_FORKLIFT" "$START_ROBOT" "$START_QUALITY_CONTROL"