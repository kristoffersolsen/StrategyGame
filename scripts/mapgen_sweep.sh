#!/usr/bin/env bash
set -euo pipefail

#PARAM=target_plate_count
#PARAM=plate_growth_rate_range
PARAM=clumping_index

#PARAM=detail_noise_amplitude
#PARAM=detail_noise_frequency
#PARAM=detail_noise_octaves

#PARAM=boundary_falloff_sigma
#PARAM=boundary_strength_noise_amplitude
#PARAM=boundary_strength_noise_frequency
#PARAM=boundary_sigma_noise_amplitude
#PARAM=boundary_sigma_noise_frequency

#PARAM=mountain_elevation_boost

#PARAM=coastal_moisture_factor
#PARAM=orographic_factor
#PARAM=base_precipitation_rate
#PARAM=evapotranspiration_rate
#PARAM=moisture_cap
#PARAM=wind_amplitude

#PARAM=river_flow_threshold
#PARAM=lake_basin_area_threshold
#PARAM=major_river_flow_threshold

MIN=0
MAX=1
STEPS=10
SEEDS=6  # run seeds 1..SEEDS

# Base preset: non-swept params come from here. Edit this file to change sweep baselines.
PRESET="tools/maptest/presets/default.ron"

TARGET_DIR="/home/kristoffer/RustroverProjects/StrategyGame"
SWEEP_OUT="${TARGET_DIR}/tmp/tests/tuning/${PARAM}"

cd "${TARGET_DIR}"

if [ -d "${SWEEP_OUT}" ]; then
    rm -rf "${SWEEP_OUT}"
    echo "Deleted ${SWEEP_OUT}"
fi

# shellcheck disable=SC2046
cargo run --release --bin maptest -- sweep \
    --param "${PARAM}" \
    --min "${MIN}" \
    --max "${MAX}" \
    --steps "${STEPS}" \
    --seeds "${SEEDS}" \
    --params "${PRESET}"
