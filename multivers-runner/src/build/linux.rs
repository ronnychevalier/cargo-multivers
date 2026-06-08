use std::convert::Infallible;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd};

use libc::{execve, fexecve};

use rustix::fd::IntoRawFd as _;
use rustix::fd::OwnedFd;
use rustix::fs::{MemfdFlags, memfd_create};

use super::{Build, Executable};

impl Executable for Build<'_> {
    unsafe fn exec(
        self,
        argc: i32,
        argv: *const *const i8,
        envp: *const *const i8,
    ) -> Result<Infallible, proc_exit::Exit> {
        let memfd_name = if argc > 0 {
            unsafe { CStr::from_ptr(*argv) }
        } else {
            c""
        };
        let mut file = memfd_create(memfd_name, MemfdFlags::CLOEXEC)
            .map(OwnedFd::into_raw_fd)
            .map(|fd| unsafe { File::from_raw_fd(fd) })
            .map_err(|_| {
                proc_exit::Code::FAILURE.with_message("Failed to create an anonymous memory file")
            })?;
        self.extract_into(&mut file).map_err(|_| {
            proc_exit::Code::FAILURE
                .with_message("Failed to write the build to an anonymous memory file")
        })?;

        let fd = file.into_raw_fd();
        let r = unsafe { fexecve(fd, argv, envp) };

        // Reaching here means `fexecve` failed (on success it never returns). On modern glibc
        // `fexecve` issues `execveat(fd, "", …, AT_EMPTY_PATH)`, which returns ENOENT for an
        // anonymous memfd under the amd64 binfmt_misc emulation used by Docker Desktop on Apple
        // Silicon. A path-based exec of the same still-valid descriptor via `/proc/self/fd` works
        // there, so fall back to it before giving up.
        if let Ok(fd_path) = CString::new(format!("/proc/self/fd/{fd}")) {
            unsafe { execve(fd_path.as_ptr(), argv, envp) };
        }

        Err(proc_exit::Exit::new(proc_exit::Code::new(r)))
    }
}
