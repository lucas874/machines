#!/usr/bin/env bash

# $1 is tmux session name
# $2 is ax log
# $3 is build log

session_name=$1
ax_log=$2
build_log=$3

DEMO_NAME="car-factory"
DISPLAY_NAME="${DEMO_NAME}-${session_name}"
APP_ID_PREFIX="com.example.${DEMO_NAME}."
APP_ID="${APP_ID_PREFIX}${session_name}"
START_STEEL_TRANSPORT="npm run start-steel-transport -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_STAMP="npm run start-stamp -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_BODY_ASSEMBLER="npm run start-body-assembler -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_CAR_BODY_CHECKER="npm run start-car-body-checker -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_PAINTER="npm run start-painter -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_BASIC_TRANSPORT="npm run start-basic-transport -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_SMART_TRANSPORT="npm run start-smart-transport -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_BASE_STATION="npm run start-base-station -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_ENGINE_INSTALLER="npm run start-engine-installer -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_ENGINE_CHECKER="npm run start-engine-checker -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_WAREHOUSE="npm run start-warehouse -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_WHEEL_INSTALLER="npm run start-wheel-installer -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_WHEEL_CHECKER="npm run start-wheel-checker -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_WINDOW_INSTALLER="npm run start-window-installer -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_WINDOW_CHECKER="npm run start-window-checker -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"
START_QUALITY_CONTROL="npm run start-quality-control -- -n ${DISPLAY_NAME} -i ${APP_ID}; exec bash"

npm run build >> $build_log 2>&1

bash ../split_and_run.sh $session_name $ax_log "$START_STEEL_TRANSPORT" "$START_STAMP" "$START_BODY_ASSEMBLER" \
    "$START_CAR_BODY_CHECKER" "$START_PAINTER" "$START_BASIC_TRANSPORT" "$START_SMART_TRANSPORT" \
    "$START_BASIC_TRANSPORT" "$START_BASIC_TRANSPORT" "$START_SMART_TRANSPORT" "$START_SMART_TRANSPORT" \
    "$START_BASE_STATION" "$START_ENGINE_INSTALLER" "$START_WAREHOUSE" "$START_ENGINE_CHECKER" "$START_WHEEL_INSTALLER" \
    "$START_WHEEL_INSTALLER" "$START_WHEEL_INSTALLER" "$START_WHEEL_INSTALLER" "$START_WHEEL_CHECKER" \
    "$START_WINDOW_INSTALLER" "$START_WINDOW_INSTALLER" "$START_WINDOW_INSTALLER" "$START_WINDOW_CHECKER" \
    "$START_QUALITY_CONTROL"