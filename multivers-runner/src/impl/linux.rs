use std::convert::Infallible;
use std::ffi::{CString, OsString};
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::os::unix::prelude::OsStringExt;

use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use nix::unistd::fexecve;

use proc_exit::prelude::*;

use crate::build::Build;

pub fn exec(build: Build, exe_filename: OsString) -> Result<Infallible, proc_exit::Exit> {
    let exe_filename = exe_filename.into_vec();

    let memfd_name = unsafe { CString::from_vec_unchecked(exe_filename) };
    let mut file = memfd_create(&memfd_name, MemFdCreateFlag::MFD_CLOEXEC)
        .map(|fd| unsafe { File::from_raw_fd(fd) })
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

    fexecve(file.into_raw_fd(), &args, &env).with_code(proc_exit::Code::FAILURE)
}
