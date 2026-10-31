#!/bin/sh
# Rebuild against the shared art, then replace this region's reviewer/sample captures.
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
review="$root/out/review/region-summit"
samples="$root/out/samples"
GLINT_ASSETS="$root/assets" cargo build --release
./tools/summit/check.sh
mkdir -p "$review" "$samples" out/summit-final
./target/release/glintblade zoneview summit --at 460,780 --out out/summit-final/bridge
./target/release/glintblade zoneview summit --at 720,640 --ticks 60 --every 30 --out out/summit-final/entrance --overview
./target/release/glintblade zoneview summit --at 720,480 --out out/summit-final/arena
./target/release/glintblade zoneview summit --at 960,400 --out out/summit-final/rim
./target/release/glintblade zoneview summit --at 720,290 --out out/summit-final/dais
./target/release/glintblade shot --zone summit --god --script 'R362 U230 A1 W20' --at 160,362,442,592,613 --out out/summit-final/play
cp out/summit-final/bridge/summit_t00090.png "$review/region_summit_bridge.png"
cp out/summit-final/entrance/summit_t00120.png "$review/region_summit_entrance.png"
cp out/summit-final/arena/summit_t00090.png "$review/region_summit_arena.png"
cp out/summit-final/rim/summit_t00090.png "$review/region_summit_rim.png"
cp out/summit-final/dais/summit_t00090.png "$review/region_summit_dais.png"
cp out/summit-final/entrance/summit_overview.png "$review/region_summit_overview.png"
cp out/summit-final/play/f00613.png "$review/region_summit_play.png"
cp "$review/region_summit_bridge.png" "$samples/region_summit_bridge.png"
cp "$review/region_summit_entrance.png" "$samples/region_summit_entrance.png"
cp "$review/region_summit_rim.png" "$samples/region_summit_rim.png"
