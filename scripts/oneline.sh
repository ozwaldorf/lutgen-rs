#!/bin/sh

# Delete repository dir
rm -rf lutgen

# Clone repository
git clone https://github.com/ozwaldorf/lutgen-rs lutgen

# Execute scripts
sh lutgen/scripts/build.sh

# Delete repository dir
rm -rf lutgen
