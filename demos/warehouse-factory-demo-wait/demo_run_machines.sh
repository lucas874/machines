#!/usr/bin/env bash

robot_ascii="
██████   ██████  ██████   ██████  ████████
██   ██ ██    ██ ██   ██ ██    ██    ██
██████  ██    ██ ██████  ██    ██    ██
██   ██ ██    ██ ██   ██ ██    ██    ██
██   ██  ██████  ██████   ██████     ██
"

# Commands to run in each window and pane
#START_R="echo 'Starting factory-robot of the Warehouse || Factory protocol.'; echo 'Press Ctrl + C to exit.'; npm run start-factory-robot;exec bash"
START_R="npm run start-factory-robot;exec bash"
START_FL="npm run start-forklift;exec bash"
START_T="npm run start-transporter;exec bash"
START_D="npm run start-door;exec bash"
START_AX="rm -rf ax-data; echo 'Silently running Actyx middleware in this window. Press Ctrl + C to exit'.; ax run 2> /dev/null"

# Start a new tmux session with the first command
tmux new-session -d -s demo "$START_AX"

# New window to run actual demo
tmux new-window -n demo-window "$START_R"

# Split into panes and run different machines
tmux split-window -h "$START_FL"

tmux select-pane -t 0
tmux split-window -v "$START_D"

tmux select-pane -t 2
tmux split-window -v "$START_T"

# Attach to the session
tmux attach-session -t demo
tmux select-window demo-window