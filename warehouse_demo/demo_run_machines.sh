#!/bin/bash
# generated mostly by chatGPT
while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean)
      pkill actyx
      rm -rf actyx-data
      gnome-terminal -- bash -c "~/Actyx/actyx"
      shift
      ;;
  esac
done
# Commands to run in each pane
START_FL="echo 'Starting forklift'; npm run start-forklift;exec bash"
START_T="echo 'Starting transporter'; npm run start-transporter;exec bash"
START_T1="echo 'Starting transporter'; npm run start-transporter1;exec bash"
START_D="echo 'Starting door'; npm run start-door;exec bash"

# Start a new tmux session with the first command
tmux new-session -d -s tiled_shells "$START_T"

# Split the window into 2 vertical panes (left and right)
tmux split-window -h "$START_FL"

# Focus on the left pane (Pane 0) and split it into 2 horizontal panes
tmux select-pane -t 0
tmux split-window -v "$START_D"

# Focus on the right pane (Pane 1) and split it into 2 horizontal panes
tmux select-pane -t 1
tmux split-window -v "$START_FL"

# Focus on the bottom-right pane (Pane 3) and split it vertically for the 5th pane
tmux select-pane -t 3
tmux split-window -v "$START_T1"

# Attach to the session
tmux attach-session -t tiled_shells
