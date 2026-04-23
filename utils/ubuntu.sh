#!/bin/sh

COMMAND=${1:-"/bin/bash"}

CONTAINER=hcbs-dev-container
SCRIPT_DIR=$(dirname $(readlink -f $0))
ROOT_DIR=$(readlink -f $SCRIPT_DIR/..)
DOCKERFILE=$SCRIPT_DIR/ubuntu.dockerfile

BUILD_DIR=${2:-"$ROOT_DIR/+build"}

docker build -t "$CONTAINER" -f "$DOCKERFILE" "$SCRIPT_DIR" || exit 1

docker run --rm \
    --user `id -u`:`id -g` \
    --workdir "/home/devContainer/+build/"\
    --volume "/etc/group:/etc/group:ro" \
    --volume "/etc/passwd:/etc/passwd:ro" \
    --volume "/etc/shadow:/etc/shadow:ro" \
    --volume "$BUILD_DIR:/home/devContainer/+build:rw" \
    "$CONTAINER" /bin/sh -c "$COMMAND"
