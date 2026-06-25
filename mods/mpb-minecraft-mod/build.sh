#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="0.1.0"
BUILD_DIR="$ROOT/build"
GENERATED_DIR="$ROOT/artifacts/generated"
ASSET_DIR="$ROOT/../../crates/mpb-assets/src"
GRADLE_BIN="${MPB_GRADLE:-}"

if [ -z "$GRADLE_BIN" ] && command -v gradle >/dev/null 2>&1; then
  GRADLE_BIN="$(command -v gradle)"
fi
if [ -z "$GRADLE_BIN" ] || [ ! -x "$GRADLE_BIN" ]; then
  echo "MPB mod production build requires Gradle. Set MPB_GRADLE=/path/to/gradle or install gradle." >&2
  exit 1
fi

rm -rf "$BUILD_DIR" "$GENERATED_DIR"
mkdir -p "$BUILD_DIR" "$GENERATED_DIR"

copy_artifact() {
  local loader="$1"
  local jar_file="$2"
  local output_file="$GENERATED_DIR/mpb-minecraft-mod-$loader.jar"
  local hex_file="$ASSET_DIR/mpb_mod_${loader}_jar.hex"

  cp "$jar_file" "$output_file"
  if command -v xxd >/dev/null 2>&1; then
    xxd -p -c 256 "$output_file" > "$hex_file"
  elif command -v node >/dev/null 2>&1; then
    node -e "const fs=require('fs'); const [input, output]=process.argv.slice(1); const hex=fs.readFileSync(input).toString('hex').match(/.{1,256}/g)?.join('\n') ?? ''; fs.writeFileSync(output, hex + (hex ? '\n' : ''));" "$output_file" "$hex_file"
  else
    echo "MPB mod production build requires xxd or node to refresh embedded hex assets." >&2
    exit 1
  fi
}

"$GRADLE_BIN" -p "$ROOT" --no-daemon ${MPB_GRADLE_EXTRA_ARGS:-} :fabric:build
"$GRADLE_BIN" -p "$ROOT" --no-daemon ${MPB_GRADLE_EXTRA_ARGS:-} :forge:build
"$GRADLE_BIN" -p "$ROOT" --no-daemon ${MPB_GRADLE_EXTRA_ARGS:-} :neoforge:build
copy_artifact fabric "$ROOT/fabric/build/libs/mpb-minecraft-mod-fabric-$VERSION.jar"
copy_artifact forge "$ROOT/forge/build/libs/mpb-minecraft-mod-forge-$VERSION.jar"
copy_artifact neoforge "$ROOT/neoforge/build/libs/mpb-minecraft-mod-neoforge-$VERSION.jar"

test_classes_dir="$BUILD_DIR/tests/classes"
mkdir -p "$test_classes_dir"
test_sources=()
while IFS= read -r source; do
  test_sources+=("$source")
done < <(find "$ROOT/common/src/main/java" "$ROOT/tests/src" -name '*.java' | sort)
javac --release 17 -encoding UTF-8 -d "$test_classes_dir" "${test_sources[@]}"
java -cp "$test_classes_dir" com.mpb.runtime.MpbMcpToolCatalogTest
java -cp "$test_classes_dir" com.mpb.runtime.MpbMcpCompatibilityTest
java -cp "$test_classes_dir" com.mpb.runtime.MpbRuntimeConfigTest
java -cp "$test_classes_dir" com.mpb.runtime.MpbGuideSchemeTest

cat > "$GENERATED_DIR/manifest.txt" <<EOF
mpb-minecraft-mod $VERSION
generated-by=mods/mpb-minecraft-mod/build.sh
loaders=fabric,forge,neoforge
EOF
