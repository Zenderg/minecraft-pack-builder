#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="${TMPDIR:-/tmp}/mpb-runtime-extractor-build"
OUT="$WORK/out"
COMMON_SRC="$ROOT/tools/runtime-extractor/src/main/java"
ASSETS_SRC="$ROOT/crates/mpb-assets/src"

rm -rf "$WORK"
mkdir -p "$OUT"

build_loader() {
  local loader="$1"
  local entry_src="$ROOT/tools/runtime-extractor/src/$loader/java"
  local stubs="$ROOT/tools/runtime-extractor/stubs/$loader"
  local resources="$ROOT/tools/runtime-extractor/resources/$loader"
  local classes="$WORK/classes-$loader"
  local jar_path="$WORK/mpb-runtime-extractor-$loader.jar"
  local hex_path="$2"

  mkdir -p "$classes"
  find "$COMMON_SRC" "$entry_src" "$stubs" -name '*.java' -print > "$WORK/sources-$loader.txt"
  javac --release 17 -d "$classes" @"$WORK/sources-$loader.txt"

  rm -rf "$WORK/jar-$loader"
  mkdir -p "$WORK/jar-$loader"
  cp -R "$resources"/. "$WORK/jar-$loader"/
  mkdir -p "$WORK/jar-$loader/com"
  cp -R "$classes/com/mpb" "$WORK/jar-$loader/com/"
  (cd "$WORK/jar-$loader" && jar cf "$jar_path" .)
  xxd -p -c 32 "$jar_path" > "$ASSETS_SRC/$hex_path"
}

build_loader "neoforge" "runtime_extractor_jar.hex"
build_loader "forge" "runtime_extractor_forge_jar.hex"
build_loader "fabric" "runtime_extractor_fabric_jar.hex"

echo "Runtime extractor jars rebuilt."
