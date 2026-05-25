#!/usr/bin/env bash
#
# Generates all Stage 3.1 (tectonics) debug images for seeds 1–8 and
# organises them into per-output-type folders under ./tmp/tests/mapgen/
# for easy cross-seed comparison.
#
# Covers every mapdump output for the completed Stage 3.1 pipeline:
#   Step 1  — plate seed placement
#   Steps 2–3 — noise-perturbed Voronoi plate map
#   Step 5  — boundary classification (convergent / divergent / transform)
#   Steps 6–7 — continuous elevation field
#   Step 8  — discrete height levels (sea-level corrected)
#   Step 9  — hills classification
#
# Usage (from workspace root):
#   ./scripts/test_seeds.sh
#
# Output layout:
#   tmp/tests/mapgen/
#     seeds/              <- 31_01_plate_seeds_px.png   (one per seed)
#     plate_map_px/       <- 31_03_plate_map_px.png
#     plate_map_hex/      <- 31_03_plate_map_hex.png
#     boundary_types/     <- 31_05_boundary_types_px.png
#     elevation/          <- 31_07_elevation_px.png
#     height_levels_px/   <- 31_08_height_levels_px.png
#     height_levels_hex/  <- 31_08_height_levels_hex.png
#     hills/              <- 31_09_hills_px.png
#
# Each image is named after the seed that produced it, e.g. seed_2.png.
# Output directories are created if absent; existing files are overwritten.

set -euo pipefail

MAPDUMP="cargo run --release --bin mapdump --"
SEEDS=(1 2 3 4 5 6 7 8)
HEX_SIZE=16

BASE="tmp/tests/mapgen"

DIR_SEEDS="$BASE/seeds"
DIR_PLATE_PX="$BASE/plate_map_px"
DIR_PLATE_HEX="$BASE/plate_map_hex"
DIR_BOUNDARY="$BASE/boundary_types"
DIR_ELEVATION="$BASE/elevation"
DIR_LEVELS_PX="$BASE/height_levels_px"
DIR_LEVELS_HEX="$BASE/height_levels_hex"
DIR_HILLS="$BASE/hills"

mkdir -p \
    "$DIR_SEEDS" \
    "$DIR_PLATE_PX" \
    "$DIR_PLATE_HEX" \
    "$DIR_BOUNDARY" \
    "$DIR_ELEVATION" \
    "$DIR_LEVELS_PX" \
    "$DIR_LEVELS_HEX" \
    "$DIR_HILLS"

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

for seed in "${SEEDS[@]}"; do
    echo "── seed $seed ──────────────────────────────"
    $MAPDUMP --seed "$seed" --output "$STAGE" --hex-size "$HEX_SIZE"

    cp "$STAGE/31_01_plate_seeds_px.png"    "$DIR_SEEDS/seed_${seed}.png"
    cp "$STAGE/31_03_plate_map_px.png"      "$DIR_PLATE_PX/seed_${seed}.png"
    cp "$STAGE/31_03_plate_map_hex.png"     "$DIR_PLATE_HEX/seed_${seed}.png"
    cp "$STAGE/31_05_boundary_types_px.png" "$DIR_BOUNDARY/seed_${seed}.png"
    cp "$STAGE/31_07_elevation_px.png"      "$DIR_ELEVATION/seed_${seed}.png"
    cp "$STAGE/31_08_height_levels_px.png"  "$DIR_LEVELS_PX/seed_${seed}.png"
    cp "$STAGE/31_08_height_levels_hex.png" "$DIR_LEVELS_HEX/seed_${seed}.png"
    cp "$STAGE/31_09_hills_px.png"          "$DIR_HILLS/seed_${seed}.png"

    rm -f "$STAGE"/*.png
done

echo ""
echo "Done. Images written to $BASE/"
echo "  seeds/             — step 1: Poisson-disk seed placement"
echo "  plate_map_px/      — steps 2–3: plate map (pixel)"
echo "  plate_map_hex/     — steps 2–3: plate map (hex, full quality)"
echo "  boundary_types/    — step 5: convergent=red, divergent=blue, transform=grey"
echo "  elevation/         — steps 6–7: continuous elevation field (Viridis)"
echo "  height_levels_px/  — step 8: discrete height levels (pixel)"
echo "  height_levels_hex/ — step 8: discrete height levels (hex)"
echo "  hills/             — step 9: hilliness (white=hilly, black=flat)"
