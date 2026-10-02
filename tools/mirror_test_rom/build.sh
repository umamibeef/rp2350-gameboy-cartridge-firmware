#!/bin/sh
# Builds mirror_test.gb with GBDK-2020. GBDK_PATH has to point to the GBDK installation, the dev
# container provides it.
set -e
cd "$(dirname "$0")"
mkdir -p build
"${GBDK_PATH:?GBDK_PATH is not set}/bin/lcc" -Wl-m -o build/mirror_test.gb mirror_test.c
echo "built $(pwd)/build/mirror_test.gb"
