#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JDK21_IMAGE="${MPB_GRADLE_CONTAINER_JDK21_IMAGE:-${MPB_GRADLE_CONTAINER_IMAGE:-gradle:8.14.3-jdk21}}"
JDK17_IMAGE="${MPB_GRADLE_CONTAINER_JDK17_IMAGE:-gradle:8.14.3-jdk17}"
CACHE_DIR="$ROOT/mods/mpb-minecraft-mod/.gradle-container-cache"

mkdir -p "$CACHE_DIR"

run_build() {
  local image="$1"
  local loaders="$2"
  local clean="$3"
  local run_tests="$4"
  local extra_args="${5:-${MPB_GRADLE_EXTRA_ARGS:--Porg.gradle.java.installations.auto-download=false}}"

  docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "$ROOT:/workspace" \
    --volume "$CACHE_DIR:/workspace/mods/mpb-minecraft-mod/.gradle-container-cache" \
    --workdir /workspace \
    --env HOME=/tmp/mpb-container-home \
    --env GRADLE_USER_HOME=/workspace/mods/mpb-minecraft-mod/.gradle-container-cache \
    --env MPB_GRADLE=gradle \
    --env MPB_GRADLE_EXTRA_ARGS="$extra_args" \
    --env MPB_MOD_LOADERS="$loaders" \
    --env MPB_CLEAN="$clean" \
    --env MPB_RUN_JAVA_TESTS="$run_tests" \
    "$image" \
    bash -lc 'mods/mpb-minecraft-mod/build.sh'
}

run_build "$JDK21_IMAGE" "fabric" "true" "false"
run_build "$JDK17_IMAGE" "forge" "false" "false"
run_build "$JDK21_IMAGE" "neoforge" "false" "true"
