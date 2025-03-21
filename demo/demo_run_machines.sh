#!/bin/bash
# this script is generated mostly by chatGPT
while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean)
      pkill ax
      rm -rf ax-data
      gnome-terminal -- bash -c "~/Actyx/./ax run"
      shift
      ;;
  esac
done
# Commands to run in each pane
START_R="echo 'Starting factory-robot'; npm run start-factory-robot;exec bash"
START_FL="echo 'Starting forklift'; npm run start-forklift;exec bash"
START_T="echo 'Starting transporter'; npm run start-transporter;exec bash"
START_QCR="echo 'Starting quality control robot'; npm run start-quality-robot;exec bash"
START_D="echo 'Starting door'; npm run start-door;exec bash"

# Start a new tmux session with the first command
tmux new-session -d -s tiled_shells "$START_R"

# Split the window into 2 vertical panes (left and right)
tmux split-window -h "$START_FL"

# Focus on the left pane (Pane 0) and split it into 2 horizontal panes
tmux select-pane -t 0
tmux split-window -v "$START_D"

# Focus on the right pane (Pane 1) and split it into 2 horizontal panes
tmux select-pane -t 1
tmux split-window -v "$START_QCR"

# Focus on the bottom-right pane (Pane 3) and split it vertically for the 5th pane
tmux select-pane -t 3
tmux split-window -v "$START_T"

# Attach to the session
tmux attach-session -t tiled_shells
