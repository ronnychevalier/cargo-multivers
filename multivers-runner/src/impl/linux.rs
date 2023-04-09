use std::convert::Infallible;
use std::ffi::{CString, OsString};
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::os::unix::prelude::OsStringExt;

use libc::fexecve;

use rustix::fd::IntoRawFd as _;
use rustix::fs::{memfd_create, MemfdFlags};

use proc_exit::prelude::*;

use crate::build::Build;

pub fn exec(build: Build, exe_filename: OsString) -> Result<Infallible, proc_exit::Exit> {
    let exe_filename = exe_filename.into_vec();

    let memfd_name = unsafe { CString::from_vec_unchecked(exe_filename) };
    let mut file = memfd_create(&memfd_name, MemfdFlags::CLOEXEC)
        .map(|fd| unsafe { File::from_raw_fd(fd.into_raw_fd()) })
        .map_err(|_| {
            proc_exit::Code::FAILURE.with_message("Failed to create an anomymous memory file")
        })?;
    build.decompress_into(&mut file).map_err(|_| {
        proc_exit::Code::FAILURE
            .with_message("Failed to write the build to an anomymous memory file")
    })?;

    let args = std::env::args_os()
        .map(|arg| CString::new(arg.into_vec()))
        .collect::<Result<Vec<CString>, _>>()
        .with_code(proc_exit::Code::FAILURE)?;
    let env = std::env::vars_os()
        .map(|(mut key, value)| {
            key.push("=");
            key.push(value);

            CString::new(key.into_vec())
        })
        .collect::<Result<Vec<CString>, _>>()
        .with_code(proc_exit::Code::FAILURE)?;

    let argv: Vec<_> = args
        .iter()
        .map(|arg| arg.as_ptr())
        .chain(Some(std::ptr::null()))
        .collect();
    let envp: Vec<_> = env
        .iter()
        .map(|arg| arg.as_ptr())
        .chain(Some(std::ptr::null()))
        .collect();

    let r = unsafe { fexecve(file.into_raw_fd(), argv.as_ptr(), envp.as_ptr()) };

    Err(proc_exit::Exit::new(proc_exit::Code::new(r)))
}
