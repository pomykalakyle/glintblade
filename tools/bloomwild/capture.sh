#!/bin/sh
# Refresh Bloomwild review frames using the embedded assets in a release build.
# Run from the checkout root; GLINT_ASSETS may point at the artist's live asset directory.
set -eu
cargo build --release
bin=./target/release/glintblade
$bin zoneview bloomwild --at 258,640 --out out/bloomwild-review/entry --overview
$bin zoneview bloomwild --at 780,700 --ticks 120 --every 30 --out out/bloomwild-review/falls
$bin zoneview bloomwild --at 664,340 --out out/bloomwild-review/terrace
$bin zoneview bloomwild --at 934,188 --walk 1,0 --ticks 120 --every 30 --out out/bloomwild-review/grove
$bin shot --zone bloomwild --pos 408,100 --peaceful --script 'W300' --out out/bloomwild-review/shrine
$bin shot --zone bloomwild --pos 400,644 --god --script 'W190 R35 A1 W20 A1 W25 A1 W20' --every 10 --out out/bloomwild-review/combat
$bin shot --zone bloomwild --pos 470,648 --peaceful --script 'W280 A1 W15 A1 W15 A1 W30' --every 5 --out out/bloomwild-review/cut
