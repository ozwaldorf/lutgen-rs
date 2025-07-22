#!/bin/sh

# Find an available container tool (docker or podman)
find_container_tool() {
    if command -v docker > /dev/null 2>&1; then
        echo "sudo docker"
    elif command -v podman > /dev/null 2>&1; then
        echo "podman"
    else
        echo "Error: Neither docker nor podman is available." >&2
        exit 1
    fi
}

CONTAINER_TOOL=$(find_container_tool)

$CONTAINER_TOOL build -t lutgen:latest https://github.com/ozwaldorf/lutgen-rs.git#main
