#!/usr/bin/env bash

# In artifact redirect ax output to log file.
CMD_START_AX="rm -rf ax-data; echo 'Silently running Actyx middleware in this window. Press Ctrl + C to quit.'; ax run &> /dev/null"

split_and_run() {
    # store remanining arguments in an array, "$@" expands to "$1" "$2" "$3" ...
    local cmds=("$@")
    local cmds_len=${#cmds[@]}
    #echo "CMDS LEN: $cmds_len"
    tmux -L "$session" new-window -n demo-window "${cmds[0]}"
    # number of commands invoked
    local n=1
    # number of 'split rounds' -- number of times we have visited all panes and split them
    local n_split_rounds=0
    while ((n < cmds_len)); do
        # direction to split
        if (( n_split_rounds % 2 == 0)); then
            local split="-h"
        else
            local split="-v"
        fi

        # i think this is more complicated than it has to be.
        # but allows us to have more panes than just splitting not changing orientation of split
        # 'split round' -- go through all panes and split them or stop when all commands have been issued
        for ((i=0; i<2**(n_split_rounds); i++)); do
            if ((n >= cmds_len)); then
                break
            fi
            tmux -L "$session" select-pane -t "%$i"
            tmux -L "$session" split-window "$split" "${cmds[n]}"
            ((n=n+1))
            done
        ((n_split_rounds=n_split_rounds+1))
    done

    tmux -L "$session" select-layout -t "$session" tiled
    tmux -L "$session" attach-session -t $session
    tmux -L "$session" select-window demo-window
}

if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <session_name> <cmd1 [cmd2 ...]>"
    exit 1
fi

# kill any running tmux processes...
pkill tmux;

session=$1
# shift arguments making the old $2 the new $1 etc.
shift

# Sketchy because ax could be some other program.... but we assume that it is called ax anyway when running it...
if pgrep ax; then
    tmux -L "$session" new-session -d -s "$session"
else
    tmux -L "$session" new-session -d -s "$session" "$CMD_START_AX"
fi

# TODO: wait for ax to start. Right now by sleeping 1 sec
sleep 1;

split_and_run "$@"
