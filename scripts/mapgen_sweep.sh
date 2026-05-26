#!/usr/bin/env bash
set -euo pipefail

#PARAM=coastal_moisture_factor
#PARAM=orographic_factor
#PARAM=base_precipitation_rate
#PARAM=evapotranspiration_rate
PARAM=moisture_cap

MIN=0.0001
MAX=0.5
STEPS=20
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