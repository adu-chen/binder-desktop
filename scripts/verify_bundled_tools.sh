#!/usr/bin/env bash
set -euo pipefail

# 发布前硬门：只校验 Binder 资源目录内已锁定的二进制，绝不通过 PATH 或 /Applications 探测系统工具。

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RESOURCE_DIR="$PROJECT_DIR/src-tauri/resources"
MANIFEST_PATH="$RESOURCE_DIR/tools/manifest.json"

[ -f "$MANIFEST_PATH" ] || { echo "Missing tool manifest: $MANIFEST_PATH" >&2; exit 1; }
json_value() {
  node -e "const o=require(process.argv[1]); console.log(process.argv.slice(2).reduce((v,k)=>v[k],o));" "$MANIFEST_PATH" "$@"
}
verify_tool() {
  local tool="$1"
  local binary_path expected_sha expected_version actual_sha actual_version
  binary_path="$RESOURCE_DIR/$(json_value "$tool" path)"
  expected_sha="$(json_value "$tool" sha256)"
  expected_version="$(json_value "$tool" version)"
  [ -x "$binary_path" ] || { echo "Missing executable bundled $tool: $binary_path" >&2; exit 1; }
  actual_sha="$(shasum -a 256 "$binary_path" | awk '{print $1}')"
  [ "$actual_sha" = "$expected_sha" ] || { echo "Bundled $tool SHA-256 mismatch" >&2; exit 1; }
  actual_version="$($binary_path --version 2>&1 | head -n 1)"
  case "$actual_version" in
    *"$expected_version"*) ;;
    *) echo "Bundled $tool version mismatch: $actual_version" >&2; exit 1 ;;
  esac
}

verify_tool pandoc
verify_tool libreoffice
echo "Bundled DOC tools verified from $RESOURCE_DIR"
