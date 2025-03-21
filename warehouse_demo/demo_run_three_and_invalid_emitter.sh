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
START_D="echo 'Starting door'; npm run start-door;exec bash"
START_INVALID="echo 'Starting invalid event emitter'; npm run start-invalid-event-emitter;exec bash"
# Start a new tmux session with the first command
tmux new-session -d -s tiled_shells "$START_T"

# Split the window into 2 vertical panes (left and right)
tmux split-window -h "$START_FL"

# Focus on the left pane (Pane 0) and split it into 2 horizontal panes
tmux select-pane -t 0
#tmux split-window -v "$START_D"
tmux split-window -v "$START_INVALID"

# Focus on the right pane (Pane 1) and split it into 2 horizontal panes
#tmux select-pane -t 2
#tmux split-window -v "$START_INVALID"

# Attach to the session
tmux attach-session -t tiled_shells
