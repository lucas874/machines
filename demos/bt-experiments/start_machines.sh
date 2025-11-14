#!/usr/bin/env bash

START_MACHINE_A="npm run start-machineA; exec bash"
START_MACHINE_D="npm run start-machineD; exec bash"
START_INTERFACE_P1="npm run start-interface-p1; exec bash"
START_INTERFACE_P2="npm run start-interface-p2; exec bash"

bash split_and_run.sh $1 "$START_MACHINE_A" "$START_MACHINE_D" "$START_INTERFACE_P1" "$START_INTERFACE_P2"