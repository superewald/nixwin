#!/usr/bin/env bash
# Builds the llvm example against the nixwin sysroot using clang-cl and
# lld-link (via $NIXWIN_LLVM_FLAGS from llvm.env), then runs the produced
# windows binary under wine when available.
#
# Prerequisites:
#   nixwin setup
#   nixwin install 17 --default
#   clang 15+ (clang-cl) and lld-link on PATH
set -euo pipefail

cd "$(dirname "$0")"

sysroot="${NIXWIN_SYSROOT:-${NIXWIN_DATA:-$HOME/.local/share/nixwin}/sysroot}"
# exports NIXWIN_SYSROOT (self-defaulting), CC, CXX, AR and NIXWIN_LLVM_FLAGS
source "$sysroot/llvm.env"

clang-cl $NIXWIN_LLVM_FLAGS main.cpp -o windows-example.exe

echo "built: $(pwd)/windows-example.exe"

if command -v wine >/dev/null 2>&1; then
    echo "running under wine:"
    wine ./windows-example.exe
else
    echo "wine not found, skipping the run"
fi