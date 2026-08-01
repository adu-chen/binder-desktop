#!/usr/bin/env bash
set -euo pipefail

# 将官方 Pandoc 3.10 arm64 发布压缩包中的单一可执行文件放入 Binder 资源目录。
# 下载、来源与 archive SHA-256 在 resources/tools/manifest.json 固定；本脚本不接受系统 PATH 的 pandoc。

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ARCHIVE_PATH="${PANDOC_ARCHIVE:?请设置 PANDOC_ARCHIVE 为官方 pandoc-3.10-arm64-macOS.zip 的绝对路径}"
TARGET_PATH="$PROJECT_DIR/src-tauri/resources/tools/pandoc/arm64/pandoc"
EXPECTED_ARCHIVE_SHA="d9cad01d96ae774a0dc8c8c45bb1ad3e4c5ff2cc2e24f45958f5f9b7974aee34"
EXPECTED_BINARY_SHA="d0a20cc85a03ec30f602c6acb568158982240431e5589e0e7c01ce80e0253493"

[ -f "$ARCHIVE_PATH" ] || { echo "Pandoc archive not found: $ARCHIVE_PATH" >&2; exit 1; }
actual_archive_sha="$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')"
[ "$actual_archive_sha" = "$EXPECTED_ARCHIVE_SHA" ] || { echo "Pandoc archive SHA-256 mismatch" >&2; exit 1; }

mkdir -p "$(dirname "$TARGET_PATH")"
unzip -p "$ARCHIVE_PATH" 'pandoc-3.10-arm64/bin/pandoc' > "$TARGET_PATH"
chmod 755 "$TARGET_PATH"
actual_binary_sha="$(shasum -a 256 "$TARGET_PATH" | awk '{print $1}')"
[ "$actual_binary_sha" = "$EXPECTED_BINARY_SHA" ] || { echo "Pandoc binary SHA-256 mismatch" >&2; exit 1; }
"$TARGET_PATH" --version | head -n 1 | grep -Fx 'pandoc 3.10' >/dev/null
echo "Bundled Pandoc 3.10 arm64 prepared: $TARGET_PATH"
