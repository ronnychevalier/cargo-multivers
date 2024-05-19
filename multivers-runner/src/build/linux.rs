use std::convert::Infallible;
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd};

use libc::fexecve;

use rustix::fd::IntoRawFd as _;
use rustix::fd::OwnedFd;
use rustix::fs::{memfd_create, MemfdFlags};

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
            unsafe { CStr::from_bytes_with_nul_unchecked(b"\0") }
        };
        let mut file = memfd_create(memfd_name, MemfdFlags::CLOEXEC)
            .map(OwnedFd::into_raw_fd)
            .map(|fd| unsafe { File::from_raw_fd(fd) })
            .map_err(|_| {
                proc_exit::Code::FAILURE.with_message("Failed to create an anomymous memory file")
            })?;
        self.extract_into(&mut file).map_err(|_| {
            proc_exit::Code::FAILURE
                .with_message("Failed to write the build to an anomymous memory file")
        })?;

        let r = unsafe { fexecve(file.into_raw_fd(), argv, envp) };

        Err(proc_exit::Exit::new(proc_exit::Code::new(r)))
    }
}
