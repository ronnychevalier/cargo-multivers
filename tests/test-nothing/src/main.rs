#![no_std]
#![no_main]
#![cfg_attr(windows, windows_subsystem = "console")]

#[used]
#[no_mangle]
pub static _fltused: i32 = 0;

#[cfg(windows)]
#[no_mangle]
pub unsafe extern "C" fn mainCRTStartup() -> ! {
    windows_sys::Win32::System::Threading::ExitProcess(0);
}

#[cfg(target_os = "linux")]
#[no_mangle]
pub unsafe extern "C" fn _start() {
    let _ = syscalls::syscall!(syscalls::Sysno::exit, 0);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_panic: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
