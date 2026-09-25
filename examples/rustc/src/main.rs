//! Cross-compiles a rust program against the nixwin windows sysroot using
//! cargo + the environment from rustc.env.
//!
//! Linkage against the most common windows import libraries is enforced
//! through the `windows-sys` crate (mirroring xwin's tests/xwin-test), plus a
//! small C++ file compiled via the cc crate (see build.rs), exercising the
//! CC/CXX/CFLAGS/LIB environment.
//!
//! Libraries exercised: kernel32, user32, gdi32, shell32, advapi32, ole32,
//! oleaut32, ws2_32
//!
//! Prerequisites:
//!   nixwin setup
//!   nixwin install 17 --default
//!   rustc/cargo with the x86_64-pc-windows-msvc target installed
//!
//! Usage: ./build.sh

use windows_sys::Win32::{
    Foundation::{LocalFree, SysAllocString, SysFreeString},
    Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, BITSPIXEL},
    Networking::WinSock::{
        closesocket, socket, WSACleanup, WSAStartup, AF_INET, INVALID_SOCKET, IPPROTO_TCP,
        SOCK_STREAM, WSADATA,
    },
    System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
    System::Environment::GetCommandLineW,
    System::LibraryLoader::GetModuleFileNameW,
    System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ},
    System::Threading::GetCurrentProcessId,
    UI::Shell::CommandLineToArgvW,
    UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
};

extern "C" {
    // from cpp/extra.cpp, compiled via the cc crate
    fn nixwin_example_pid() -> u32;
}

fn wstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

fn main() {
    unsafe {
        // kernel32
        println!("pid: {}", GetCurrentProcessId());

        let mut module = [0u16; 512];
        GetModuleFileNameW(std::ptr::null_mut(), module.as_mut_ptr(), module.len() as u32);
        println!("module: {}", wide_to_string(&module));

        // user32
        println!(
            "screen: {}x{}",
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN)
        );

        // gdi32
        let hdc = GetDC(std::ptr::null_mut());
        println!("bpp: {}", GetDeviceCaps(hdc, BITSPIXEL as i32));
        ReleaseDC(std::ptr::null_mut(), hdc);

        // shell32
        let mut argc = 0;
        let argv = CommandLineToArgvW(GetCommandLineW(), &mut argc);
        println!("argc: {argc}");
        LocalFree(argv as *mut _);

        // advapi32
        let key = wstr("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
        let value = wstr("ProductName");
        let mut buf = [0u16; 256];
        let mut size = (buf.len() * 2) as u32;
        let err = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size,
        );
        if err == 0 {
            println!("windows: {}", wide_to_string(&buf));
        }

        // ole32
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        CoUninitialize();

        // oleaut32
        let bstr = SysAllocString(wstr("nixwin").as_ptr());
        if !bstr.is_null() {
            let len = (0..).take_while(|&i| *bstr.add(i) != 0).count();
            println!("bstr: {}", String::from_utf16_lossy(std::slice::from_raw_parts(bstr, len)));
            SysFreeString(bstr);
        }

        // ws2_32
        let mut wsa: WSADATA = std::mem::zeroed();
        if WSAStartup(0x0202, &mut wsa) == 0 {
            let sock = socket(AF_INET as i32, SOCK_STREAM, IPPROTO_TCP as i32);
            assert_ne!(sock, INVALID_SOCKET, "failed to open a socket");
            println!("socket: {sock}");
            closesocket(sock);
            WSACleanup();
        }

        // C++ code compiled through the cc crate
        println!("cc pid: {}", nixwin_example_pid());
    }
}