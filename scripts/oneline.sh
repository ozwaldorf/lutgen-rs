#!/bin/sh

# Delete repository dir
rm -rf lutgen

# Clone repository
git clone https://github.com/ozwaldorf/lutgen-rs lutgen

# Execute scripts
sh lutgen/scripts/build.sh

# Delete repository dir
rm -rf lutgen

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

# Determine which container tool to use
CONTAINER_TOOL=$(find_container_tool)

$CONTAINER_TOOL image rm rust:alpine
$CONTAINER_TOOL image rm alpine:latest
