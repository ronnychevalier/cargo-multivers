use std::convert::Infallible;
use std::ffi::{CStr, c_char};
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd};

use libc::fexecve;

use rustix::fd::IntoRawFd as _;
use rustix::fd::OwnedFd;
use rustix::fs::{MemfdFlags, memfd_create};

use super::{Build, Executable};

impl Executable for Build<'_> {
    unsafe fn exec(
        self,
        argc: i32,
        argv: *const *const c_char,
        envp: *const *const c_char,
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
        let mut error = std::io::Error::last_os_error();

        // `fexecve` returns only on failure. glibc implements it as
        // `execveat(fd, "", …, AT_EMPTY_PATH)`; when the build is run through an interpreter
        // (a binfmt_misc handler such as an emulator, or a "#!" script), the kernel re-opens the
        // descriptor for that interpreter, but a close-on-exec descriptor is already closed by
        // then, so the call fails with ENOENT (see the BUGS section of execveat(2)). Clear
        // close-on-exec and retry so the descriptor survives into the interpreter. This only
        // happens on the interpreter path, so the executed program inherits the descriptor only
        // in that case.
        if error.raw_os_error() == Some(libc::ENOENT) {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if flags >= 0
                && unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) } >= 0
            {
                unsafe { fexecve(fd, argv, envp) };
                error = std::io::Error::last_os_error();
            }
        }

        Err(proc_exit::Code::FAILURE.with_message(format!("Failed to execute the build: {error}")))
    }
}
