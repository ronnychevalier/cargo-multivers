#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

ARGS="${INPUT_OTHER_ARGS:-}"
if [[ -n "${INPUT_MANIFEST_PATH:-}" ]]; then
    ARGS+=" --manifest-path ${INPUT_MANIFEST_PATH}"
fi
if [[ -n "${INPUT_TARGET:-}" ]]; then
    ARGS+=" --target ${INPUT_TARGET}"
fi
if [[ -n "${INPUT_RUNNER_VERSION:-}" ]]; then
    ARGS+=" --runner-version ${INPUT_RUNNER_VERSION}"
fi
if [[ -n "${INPUT_PROFILE:-}" ]]; then
    ARGS+=" --profile ${INPUT_PROFILE}"
fi
if [[ -n "${INPUT_OUT_DIR:-}" ]]; then
    ARGS+=" --out-dir ${INPUT_OUT_DIR}"
fi

cargo multivers ${ARGS} -- ${INPUT_BUILD_ARGS:-}