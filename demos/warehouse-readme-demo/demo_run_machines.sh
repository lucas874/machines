#!/usr/bin/env bash

# Commands to run in each window and pane
START_TR="echo 'Starting transport robot'; npm run start-transport-robot;exec bash"
START_W="echo 'Starting warehouse'; npm run start-warehouse;exec bash"
START_AR="echo 'Starting assembly robot'; npm run start-assembly-robot;exec bash"
START_AX="rm -rf ax-data; echo 'Silently running Actyx middleware in this window. Press Ctrl + C to exit'.; ax run 2> /dev/null"

# Start a new tmux session with the first command
tmux new-session -d -s demo "$START_AX"

# New window to run actual demo
tmux new-window -n demo-window "$START_TR"

# Split into panes and run different machines
tmux split-window -h "$START_TR"

tmux select-pane -t 0
tmux split-window -v "$START_TR"

tmux select-pane -t 1
tmux split-window -v "$START_AR"

tmux select-pane -t 3
tmux split-window -v "$START_W"

# Attach to the session
tmux attach-session -t demo
