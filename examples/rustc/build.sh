#!/usr/bin/env bash
# Builds the rustc example against the nixwin sysroot using cargo and the
# environment from rustc.env (CC/CXX/AR, CFLAGS, LIB and RUSTFLAGS), then runs
# the produced windows binary under wine when available.
#
# Prerequisites:
#   nixwin setup
#   nixwin install 17 --default
#   rustc/cargo with the x86_64-pc-windows-msvc target installed
set -euo pipefail

cd "$(dirname "$0")"

sysroot="${NIXWIN_SYSROOT:-${NIXWIN_DATA:-$HOME/.local/share/nixwin}/sysroot}"
# exports NIXWIN_SYSROOT (self-defaulting), CC, CXX, AR, CFLAGS, LIB and RUSTFLAGS
source "$sysroot/rustc.env"

cargo build --target x86_64-pc-windows-msvc

exe="$(pwd)/target/x86_64-pc-windows-msvc/debug/rustc-example.exe"
echo "built: $exe"

if command -v wine >/dev/null 2>&1; then
    echo "running under wine:"
    wine "$exe"
else
    echo "wine not found, skipping the run"
fi