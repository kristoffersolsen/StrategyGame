#!/usr/bin/env bash
#
# Generates plate-map debug images for seeds 1–8 and organises them
# into per-type folders under ./tmp/tests/mapgen/ for easy review.
#
# Usage (from workspace root):
#   ./scripts/test_seeds.sh
#
# Output layout:
#   tmp/tests/mapgen/seeds/          <- 31_01_plate_seeds_px.png  (one per seed)
#   tmp/tests/mapgen/plate_map_px/   <- 31_03_plate_map_px.png
#   tmp/tests/mapgen/plate_map_hex/  <- 31_03_plate_map_hex.png
#
# Each image is named after the seed that produced it, e.g. seed_2.png.
# The output directories are created if they do not exist.
# Existing images with the same name are overwritten.

set -euo pipefail

MAPDUMP="cargo run --release --bin mapdump --"
SEEDS=(1 2 3 4 5 6 7 8)
HEX_SIZE=16

# Output roots
BASE="tmp/tests/mapgen"
DIR_SEEDS="$BASE/seeds"
DIR_PLATE_PX="$BASE/plate_map_px"
DIR_PLATE_HEX="$BASE/plate_map_hex"

mkdir -p "$DIR_SEEDS" "$DIR_PLATE_PX" "$DIR_PLATE_HEX"

# Temporary staging directory; cleaned up after each seed.
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

for seed in "${SEEDS[@]}"; do
    echo "── seed $seed ──────────────────────────────"
    $MAPDUMP --seed "$seed" --output "$STAGE" --hex-size "$HEX_SIZE"

    cp "$STAGE/31_01_plate_seeds_px.png"  "$DIR_SEEDS/seed_${seed}.png"
    cp "$STAGE/31_03_plate_map_px.png"    "$DIR_PLATE_PX/seed_${seed}.png"
    cp "$STAGE/31_03_plate_map_hex.png"   "$DIR_PLATE_HEX/seed_${seed}.png"

    # Clear the staging directory for the next seed.
    rm -f "$STAGE"/*.png
done

echo ""
echo "Done. Images written to $BASE/"
echo "  seeds/          — Poisson-disk seed placement"
echo "  plate_map_px/   — plate map (pixel, fast)"
echo "  plate_map_hex/  — plate map (hex, full quality)"