# Cross-compiles the windows example against the nixwin sysroot using the
# generated toolchain (clang-cl + lld-link, see cmake.env).
#
# The sysroot is resolved from (in order): the NIXWIN_SYSROOT CMake variable,
# the NIXWIN_SYSROOT environment variable, or the directory of toolchain.cmake.
#
# The target architecture defaults to the primary architecture of the sysroot
# and can be overridden with -DTARGET_PLATFORM=Win32 (x86) or Win64 (x86_64).
cmake_minimum_required(VERSION 3.19)

project(nixwin-windows-example LANGUAGES CXX)

add_executable(windows-example main.cpp)

# link against the most common windows import libraries, resolved through the
# nixwin sysroot via the toolchain's /winsysroot lib paths
target_link_libraries(windows-example PRIVATE
    kernel32
    user32
    gdi32
    shell32
    advapi32
    ole32
    oleaut32
    uuid
    ws2_32
)