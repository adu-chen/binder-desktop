#!/bin/bash
# Binder LO macOS slim-down postprocessor
# Strips components not needed for headless PDF conversion.
# Preserves: Writer, Calc, Impress (sw/sc/sd), CJK+Latin locale, all filters.
# Usage: postprocess_lo_macos.sh <LO_APP_PATH>
set -euo pipefail

APP="$1"
FRAMEWORKS="$APP/Contents/Frameworks"
RESOURCES="$APP/Contents/Resources"

TARGET_LOCALES="en-US zh-CN zh-TW ja ko"

# ============================================================
# 0. Suppress Dock icon for headless operation
# ============================================================
PLIST="$APP/Contents/Info.plist"
if [ -f "$PLIST" ]; then
    /usr/libexec/PlistBuddy -c "Add :LSUIElement bool true" "$PLIST" 2>/dev/null ||     /usr/libexec/PlistBuddy -c "Set :LSUIElement true" "$PLIST" 2>/dev/null || true
    echo "[binder:lo-slim] LSUIElement=true (no Dock icon)"
fi

"en-US zh-CN zh-TW ja ko"
echo "[binder:lo-slim] target: $APP"

# ============================================================
# 1. Python framework (headless conversion never uses Python macros)
# ============================================================
PYFRAMEWORK="$FRAMEWORKS/LibreOfficePython.framework"
if [ -d "$PYFRAMEWORK" ]; then
    size=$(du -sm "$PYFRAMEWORK" | cut -f1)
    rm -rf "$PYFRAMEWORK"
    echo "[binder:lo-slim] removed LibreOfficePython.framework ($size MB)"
fi
PYLOADER="$FRAMEWORKS/libpythonloaderlo.dylib"
if [ -f "$PYLOADER" ]; then
    rm -f "$PYLOADER"
    echo "[binder:lo-slim] removed libpythonloaderlo.dylib"
fi

# ============================================================
# 2. Non-target locale registry resources
# ============================================================
REGISTRY_RES="$RESOURCES/registry/res"
if [ -d "$REGISTRY_RES" ]; then
    removed=0
    for xcd in "$REGISTRY_RES/"*.xcd; do
        [ -f "$xcd" ] || continue
        bn=$(basename "$xcd" .xcd)
        keep=0
        for loc in $TARGET_LOCALES; do
            norm=$(echo "$loc" | tr '-' '_')
            if echo "$bn" | grep -qi "$norm\|$loc"; then
                keep=1
                break
            fi
        done
        # Also keep files without locale suffix (shared configs)
        if ! echo "$bn" | grep -qiE '_[a-z]{2}([_-][A-Z]{2})?$'; then
            keep=1
        fi
        if [ $keep -eq 0 ]; then
            rm -f "$xcd"
            removed=$((removed + 1))
        fi
    done
    echo "[binder:lo-slim] removed $removed non-target locale registry files"
fi

# ============================================================
# 3. Spelling dictionaries — KEPT (required for linguistic service init)
# ============================================================
echo "[binder:lo-slim] dictionaries kept (linguistic init requirement)"

# ============================================================
# 4. AutoText (only en-US needed)
# ============================================================
for at in "$RESOURCES/autotext/"*/; do
    [ -d "$at" ] || continue
    dn=$(basename "$at")
    if [ "$dn" != "en-US" ]; then
        rm -rf "$at"
        echo "[binder:lo-slim] removed autotext/$dn"
    fi
done

# ============================================================
# 5. Help / wizard / template files
# ============================================================
for dir in help wizard; do
    if [ -d "$RESOURCES/$dir" ]; then
        size=$(du -sm "$RESOURCES/$dir" | cut -f1)
        rm -rf "$RESOURCES/$dir"
        echo "[binder:lo-slim] removed $dir ($size MB)"
    fi
done

# ============================================================
# 6. Gallery / labels / numbering templates
# ============================================================
for dir in gallery labels numbertext; do
    if [ -d "$RESOURCES/$dir" ]; then
        size=$(du -sm "$RESOURCES/$dir" | cut -f1)
        rm -rf "$RESOURCES/$dir"
        echo "[binder:lo-slim] removed $dir ($size MB)"
    fi
done

# ============================================================
# 7. Non-essential config modules (sw/sc/sd kept)
# ============================================================
# Only keep Writer, Calc, and Impress UI config; remove Base, Draw, Math, etc.
KEEP_MODULES="swriter scalc simpress StartModule"
if [ -d "$RESOURCES/config/soffice.cfg/modules" ]; then
    for mod_dir in "$RESOURCES/config/soffice.cfg/modules/"*/; do
        [ -d "$mod_dir" ] || continue
        dn=$(basename "$mod_dir")
        keep=0
        for km in $KEEP_MODULES; do
            if echo "$dn" | grep -qi "$km"; then
                keep=1
                break
            fi
        done
        if [ $keep -eq 0 ]; then
            rm -rf "$mod_dir"
            echo "[binder:lo-slim] removed module config $dn"
        fi
    done
fi

# ============================================================
# Summary
# ============================================================
final_mb=$(du -sm "$APP" | cut -f1)
echo "[binder:lo-slim] final size: ${final_mb}MB"
