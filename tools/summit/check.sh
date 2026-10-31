#!/bin/sh
# Run from the worktree root after building with the intended GLINT_ASSETS folder.
set -eu
mkdir -p out/summit-check
SDKROOT="${SDKROOT:-/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk}" rustc --edition 2024 -O tools/summit/check.rs \
    --extern glintblade=target/release/libglintblade.rlib \
    -L dependency=target/release/deps -o out/summit-check/check
./out/summit-check/check
