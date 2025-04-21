#!/usr/bin/env bash

# Commands to run in each window and pane
START_FL="echo 'Starting forklift'; npm run start-forklift;exec bash"
START_T="echo 'Starting transporter'; npm run start-transporter;exec bash"
START_D="echo 'Starting door'; npm run start-door;exec bash"
START_INVALID="echo 'Starting invalid event emitter'; npm run start-invalid-event-emitter;exec bash"
START_AX="rm -rf ax-data; echo 'Silently running Actyx middleware in this window. Press Ctrl + C to exit'.; ax run 2> /dev/null"

# Start a new tmux session with the first command
tmux new-session -d -s demo "$START_AX"

# Window to run actual demo
tmux new-window -n demo-window "$START_T"

# Split into panes running different machines + and the invalid event emitter
tmux split-window -h "$START_FL"
tmux select-pane -t 0
tmux split-window -v "$START_INVALID"

# Attach to the session
tmux attach-session -t demo
