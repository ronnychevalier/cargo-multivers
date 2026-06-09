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
        unsafe { fexecve(fd, argv, envp) };

        // `fexecve` returns only on failure. glibc implements it as
        // `execveat(fd, "", …, AT_EMPTY_PATH)`; when the build is run through an interpreter
        // registered with binfmt_misc (e.g. an emulator), the kernel hands the descriptor to
        // that interpreter, but a close-on-exec descriptor is already closed by then, so the
        // call fails with ENOENT (see the BUGS section of execveat(2)). Only in that case, retry
        // by path: the interpreter resolves `/proc/self/fd` at exec time, and the descriptor
        // stays close-on-exec so it is not leaked into the executed program.
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT)
            && let Ok(fd_path) = CString::new(format!("/proc/self/fd/{fd}"))
        {
            unsafe { execve(fd_path.as_ptr(), argv, envp) };
        }

        let error = std::io::Error::last_os_error();
        Err(proc_exit::Code::FAILURE.with_message(format!("Failed to execute the build: {error}")))
    }
}
