#!/usr/bin/env bash

version="Warehouse"
START_TRANSPORT="npm run start-transport -- $version; exec bash"
START_DOOR="npm run start-door -- $version; exec bash"
START_FORKLIFT="npm run start-forklift -- $version; exec bash"
START_ROBOT="npm run start-factory-robot -- $version; exec bash"

npm run build

bash ../split_and_run.sh $1 "$START_TRANSPORT" "$START_DOOR" "$START_FORKLIFT"