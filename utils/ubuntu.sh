#!/bin/sh

COMMAND=${1:-"/bin/bash"}
CONTAINER=hcbs-dev-container
SCRIPT_DIR=$(dirname $(readlink -f $0))
ROOT_DIR=$(readlink -f $SCRIPT_DIR/..)
DOCKERFILE=$SCRIPT_DIR/ubuntu.dockerfile

docker build -t "$CONTAINER" -f "$DOCKERFILE" "$SCRIPT_DIR" || exit 1

docker run -ti --rm \
    --user `id -u`:`id -g` \
    --workdir "/home/devContainer/+build/busybox"\
    --volume "/etc/group:/etc/group:ro" \
    --volume "/etc/passwd:/etc/passwd:ro" \
    --volume "/etc/shadow:/etc/shadow:ro" \
    --volume "$ROOT_DIR:/home/devContainer:ro" \
    --volume "$ROOT_DIR/+build/busybox:/home/devContainer/+build/busybox:rw" \
    "$CONTAINER" /bin/sh -c "$COMMAND"
