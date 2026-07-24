#!/usr/bin/env bash
# Build the pinned LibreOffice source for Binder's S1 no-GUI feasibility gate.
# Usage: scripts/build_lo_macos.sh /absolute/path/to/libreoffice-core
set -euo pipefail

SOURCE_DIR="${1:?usage: scripts/build_lo_macos.sh /absolute/path/to/libreoffice-core}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCH_FILES=(
  "$REPO_ROOT/patches/libreoffice/binder_font_identity.patch"
  "$REPO_ROOT/patches/libreoffice/binder_tarball_mirror_fallback.patch"
)
EXPECTED_COMMIT="30742500f2d3eb4366ac312fa33d3dcabdb3eba5"
# Keep the default conservative. A human may explicitly lower it for a
# constrained feasibility build via BINDER_LO_MIN_FREE_GIB; this is not used
# for release builds.
MIN_FREE_GIB="${BINDER_LO_MIN_FREE_GIB:-50}"
BUILD_JOBS="${BINDER_LO_BUILD_JOBS:-$(sysctl -n hw.ncpu)}"
LODE_ROOT="${BINDER_LO_LODE_HOME:-}"
SKIP_CONFIGURE="${BINDER_LO_SKIP_CONFIGURE:-0}"

# This is Binder's document-preview build, not a general-purpose desktop
# office suite. Keep Writer, Impress, Calc, PDF export and Quartz rendering,
# while excluding components that cannot participate in DOCX/PPTX/XLSX-to-PDF
# conversion.  In particular, --disable-pdfimport only removes PDF *input*;
# it does not remove the PDF export path used by the preview runtime.
LO_CONFIGURE_ARGS=(
  --with-distro=LibreOfficeMacOSX
  --without-java
  --without-help
  --without-doxygen
  --disable-odk
  --disable-online-update
  --disable-mariadb-sdbc
  --disable-firebird-sdbc
  --disable-postgresql-sdbc
  --disable-ldap
  --disable-gpgmepp
  --without-webdav
  --disable-zxing
  --disable-ext-nlpsolver
  --disable-pdfimport
  --disable-pdfium
  --disable-poppler
  --disable-sdremote
  --disable-librelogo
  --disable-lotuswordpro
  # Third-party fonts are deliberately excluded from the feasibility build.
  # The release pipeline must later inject the licensed, hash-pinned
  # Binder font set and the FontPlan manifest before packaging.
  --without-fonts
)

fail() {
  echo "[binder:lo-build] $*" >&2
  exit 1
}

[[ -d "$SOURCE_DIR/.git" ]] || fail "not a LibreOffice git checkout: $SOURCE_DIR"
for patch_file in "${PATCH_FILES[@]}"; do
  [[ -f "$patch_file" ]] || fail "missing pinned Binder patch: $patch_file"
done
[[ "$MIN_FREE_GIB" =~ ^[0-9]+$ ]] || fail "BINDER_LO_MIN_FREE_GIB must be an integer"
[[ "$BUILD_JOBS" =~ ^[1-9][0-9]*$ ]] || fail "BINDER_LO_BUILD_JOBS must be a positive integer"
[[ "$SKIP_CONFIGURE" == "0" || "$SKIP_CONFIGURE" == "1" ]] || fail \
  "BINDER_LO_SKIP_CONFIGURE must be 0 or 1"
[[ -n "$LODE_ROOT" ]] || fail \
  "BINDER_LO_LODE_HOME is required; build the official LibreOffice development environment (LODE) first"
[[ -d "$LODE_ROOT" ]] || fail "BINDER_LO_LODE_HOME is not a directory: $LODE_ROOT"
[[ -x "$LODE_ROOT/opt/bin/pkgconf" ]] || fail \
  "LODE pkgconf is missing: $LODE_ROOT/opt/bin/pkgconf; finish the LODE setup before building LibreOffice"
[[ -x "$LODE_ROOT/opt/bin/ninja" ]] || fail \
  "LODE ninja is missing: $LODE_ROOT/opt/bin/ninja; provide an isolated Ninja before building LibreOffice"

# LibreOffice deliberately rejects a host pkg-config/pkgconf with installed
# packages. Keep the build-tool environment isolated while retaining only the
# selected Xcode compiler toolchain from macOS.
export LODE_HOME="$LODE_ROOT"
# Do not append the interactive user PATH. In particular, a Homebrew meson
# launcher can target a different Python and prevent LO from selecting its
# hash-pinned internal Meson source. Xcode is addressed by absolute path by
# LO's macOS configuration, while these are the required system utilities.
export PATH="$LODE_ROOT/opt/bin:/usr/bin:/bin:/usr/sbin:/sbin"

# LODE provides ccache, but its default macOS cache location is user-scoped.
# Keep the cache beside the isolated build tools so configuration probes never
# depend on a writable host-home directory. Limit it for the constrained S1
# feasibility build; cache contents are not runtime assets.
export CCACHE_DIR="${BINDER_LO_CCACHE_DIR:-$LODE_ROOT/ccache}"
export CCACHE_TEMPDIR="${BINDER_LO_CCACHE_TEMPDIR:-$LODE_ROOT/ccache-tmp}"
export CCACHE_MAXSIZE="${BINDER_LO_CCACHE_MAXSIZE:-2G}"
mkdir -p "$CCACHE_DIR" "$CCACHE_TEMPDIR"

# The source build fetches hash-pinned upstream tarballs and submodules. Do
# not inherit a loopback proxy injected by a non-interactive session: it is
# not part of the reproducible toolchain and may not exist for this process.
unset http_proxy https_proxy all_proxy HTTP_PROXY HTTPS_PROXY ALL_PROXY no_proxy NO_PROXY

ACTIVE_DEVELOPER_DIR="$(xcode-select -p 2>/dev/null || true)"
[[ "$ACTIVE_DEVELOPER_DIR" == "/Applications/Xcode.app/Contents/Developer" ]] || fail \
  "full Xcode 14.3+ is required; Command Line Tools alone are not a supported LO macOS build environment"
xcodebuild -version >/dev/null 2>&1 || fail "xcodebuild is unavailable"
GNU_MAKE=""
for candidate in "$LODE_ROOT/opt/bin/gmake" "$LODE_ROOT/opt/bin/make"; do
  if [[ -x "$candidate" ]]; then
    GNU_MAKE="$candidate"
    break
  fi
done
[[ -n "$GNU_MAKE" ]] || fail "GNU Make 4.0+ is missing from LODE: $LODE_ROOT/opt/bin"
"$GNU_MAKE" --version | head -n 1 | grep -Eq 'GNU Make (4|[5-9])\.' || fail \
  "GNU Make 4.0+ is required; found $($GNU_MAKE --version | head -n 1)"
export MAKE="$GNU_MAKE"

SOURCE_COMMIT="$(git -C "$SOURCE_DIR" rev-parse HEAD)"
[[ "$SOURCE_COMMIT" == "$EXPECTED_COMMIT" ]] || fail \
  "unexpected LO source commit: $SOURCE_COMMIT (expected $EXPECTED_COMMIT)"

AVAILABLE_KIB="$(df -Pk "$SOURCE_DIR" | awk 'NR == 2 { print $4 }')"
MIN_FREE_KIB=$((MIN_FREE_GIB * 1024 * 1024))
(( AVAILABLE_KIB >= MIN_FREE_KIB )) || fail \
  "need at least ${MIN_FREE_GIB}GiB free at the source volume; have $((AVAILABLE_KIB / 1024 / 1024))GiB"

# The feasibility build intentionally excludes Java components: Java is not a
# Binder runtime dependency and must not reintroduce a host-JRE requirement.
for patch_file in "${PATCH_FILES[@]}"; do
  if git -C "$SOURCE_DIR" apply --check --reverse "$patch_file" 2>/dev/null; then
    echo "[binder:lo-build] $(basename "$patch_file") already applied"
  else
    git -C "$SOURCE_DIR" apply --check "$patch_file" || fail "Binder patch does not apply cleanly: $patch_file"
    git -C "$SOURCE_DIR" apply "$patch_file"
  fi
done
git -C "$SOURCE_DIR" diff --check

pushd "$SOURCE_DIR" >/dev/null
if [[ "$SKIP_CONFIGURE" == "1" ]]; then
  [[ -f config_host.mk ]] || fail \
    "cannot skip configuration: config_host.mk is missing"
  echo "[binder:lo-build] resuming existing verified configuration"
else
  ./autogen.sh "${LO_CONFIGURE_ARGS[@]}"
fi
# `build` is LibreOffice's documented default goal and does not execute the
# separate `check` target. Do not invent a "nocheck" target: that would hide
# a target-resolution error rather than make the build safer.
"$GNU_MAKE" build -j"$BUILD_JOBS"
popd >/dev/null

echo "[binder:lo-build] build completed; run the dedicated no-GUI conversion verification before packaging"
