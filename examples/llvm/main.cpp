// Compiles and links a small C++ program against the nixwin windows sysroot,
// forcing linkage against the most common windows import libraries via
// `#pragma comment(lib, ...)` directives, mirroring how the rustc example
// enforces linkage with the windows-sys crate.
//
// Libraries exercised: kernel32, user32, gdi32, shell32, advapi32, ole32,
// oleaut32, uuid, ws2_32
//
// Prerequisites:
//   nixwin setup
//   nixwin install 17 --default
//   clang 15+ (clang-cl) and lld-link on PATH
//
// Usage: ./build.sh

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <winsock2.h>
#include <windows.h>
#include <shellapi.h>
#include <objbase.h>
#include <oleauto.h>
#include <cstdio>

// link against the most common import libraries from the sysroot
#pragma comment(lib, "kernel32")
#pragma comment(lib, "user32")
#pragma comment(lib, "gdi32")
#pragma comment(lib, "shell32")
#pragma comment(lib, "advapi32")
#pragma comment(lib, "ole32")
#pragma comment(lib, "oleaut32")
#pragma comment(lib, "uuid")
#pragma comment(lib, "ws2_32")

int main() {
    // kernel32
    std::printf("pid: %lu\n", static_cast<unsigned long>(GetCurrentProcessId()));

    wchar_t module[MAX_PATH] = {};
    GetModuleFileNameW(nullptr, module, MAX_PATH);
    std::printf("module: %ls\n", module);

    // user32
    std::printf("screen: %dx%d\n", GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));

    // gdi32
    HDC hdc = GetDC(nullptr);
    std::printf("bpp: %d\n", GetDeviceCaps(hdc, BITSPIXEL));
    ReleaseDC(nullptr, hdc);

    // shell32
    int argc = 0;
    wchar_t** argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    std::printf("argc: %d\n", argc);
    LocalFree(argv);

    // advapi32
    wchar_t product[256] = {};
    DWORD size = sizeof(product);
    if (RegGetValueW(
            HKEY_LOCAL_MACHINE,
            L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
            L"ProductName",
            RRF_RT_REG_SZ,
            nullptr,
            product,
            &size
        ) == ERROR_SUCCESS) {
        std::printf("windows: %ls\n", product);
    }

    // ole32 + oleaut32
    CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

    GUID guid{};
    CoCreateGuid(&guid);
    std::printf(
        "guid: %08lx-%04x-%04x\n",
        static_cast<unsigned long>(guid.Data1),
        guid.Data2,
        guid.Data3
    );

    // uuid: referencing the IID constant forces resolution from uuid.lib
    std::printf("iid_iunknown: %08lx\n", static_cast<unsigned long>(IID_IUnknown.Data1));

    BSTR str = SysAllocString(L"nixwin");
    std::printf("bstr: %ls\n", str);
    SysFreeString(str);

    CoUninitialize();

    // ws2_32
    WSADATA wsa{};
    if (WSAStartup(MAKEWORD(2, 2), &wsa) == 0) {
        SOCKET socket_handle = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
        std::printf("socket: %llu\n", static_cast<unsigned long long>(socket_handle));
        closesocket(socket_handle);
        WSACleanup();
    }

    return 0;
}