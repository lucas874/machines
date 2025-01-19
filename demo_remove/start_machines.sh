#!/bin/bash
# from chatGPT
while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean)
      pkill actyx
      rm -rf actyx-data
      ls ~/Actyx
      gnome-terminal -- bash -c "~/Actyx/actyx"
      shift
      ;;
  esac
done
# List of commands to run in new terminal windows
commands=(
  "echo 'Starting factory-robot'; npm run start-factory-robot"
  "echo 'Starting forklift'; npm run start-forklift"
  "echo 'Starting transporter'; npm run start-transporter"
  "echo 'Starting door'; npm run start-door"
  "echo 'Starting quality control robot'; npm run start-quality-robot"
)

# Loop through the commands and open each in a new terminal window
for cmd in "${commands[@]}"; do
  gnome-terminal -- bash -c "$cmd"
done
