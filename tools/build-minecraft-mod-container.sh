#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${MPB_GRADLE_CONTAINER_IMAGE:-gradle:8.14.3-jdk21}"
CACHE_DIR="$ROOT/mods/mpb-minecraft-mod/.gradle-container-cache"

mkdir -p "$CACHE_DIR"

docker run --rm \
  --user "$(id -u):$(id -g)" \
  --volume "$ROOT:/workspace" \
  --volume "$CACHE_DIR:/workspace/mods/mpb-minecraft-mod/.gradle-container-cache" \
  --workdir /workspace \
  --env HOME=/tmp/mpb-container-home \
  --env GRADLE_USER_HOME=/workspace/mods/mpb-minecraft-mod/.gradle-container-cache \
  --env MPB_GRADLE=gradle \
  --env MPB_GRADLE_EXTRA_ARGS="${MPB_GRADLE_EXTRA_ARGS:--Porg.gradle.java.installations.auto-download=true}" \
  "$IMAGE" \
  bash -lc 'mods/mpb-minecraft-mod/build.sh'
