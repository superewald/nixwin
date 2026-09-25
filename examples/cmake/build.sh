#!/usr/bin/env bash
# Builds the cmake example against the nixwin sysroot via the generated
# CMake toolchain (sourced through cmake.env), then runs the produced
# windows binary under wine when available.
#
# Prerequisites:
#   nixwin setup
#   nixwin install 17 --default
#   cmake and clang 15+ (clang-cl) on PATH
set -euo pipefail

cd "$(dirname "$0")"

sysroot="${NIXWIN_SYSROOT:-${NIXWIN_DATA:-$HOME/.local/share/nixwin}/sysroot}"
# exports CMAKE_TOOLCHAIN_FILE and CMAKE_CLANG_VFS_OVERLAY pointing at the sysroot
source "$sysroot/cmake.env"

cmake -B build
cmake --build build

exe="$(pwd)/build/windows-example.exe"
echo "built: $exe"

if command -v wine >/dev/null 2>&1; then
    echo "running under wine:"
    wine "$exe"
else
    echo "wine not found, skipping the run"
fi