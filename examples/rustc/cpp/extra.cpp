// Compiled through the cc crate via the CC/CXX/CFLAGS environment from
// rustc.env, exercising the sysroot's clang-cl setup.
#include <windows.h>

extern "C" unsigned long nixwin_example_pid(void) {
    return GetCurrentProcessId();
}