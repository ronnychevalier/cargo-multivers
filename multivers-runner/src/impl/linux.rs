use std::convert::Infallible;
use std::ffi::{CString, OsString};
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::os::unix::prelude::OsStringExt;

use anyhow::{Context, Result};

use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use nix::unistd::fexecve;

use crate::build::Build;

pub fn exec(build: Build, exe_filename: OsString) -> Result<Infallible> {
    let exe_filename = exe_filename.into_vec();

    let memfd_name = unsafe { CString::from_vec_unchecked(exe_filename) };
    let mut file = memfd_create(&memfd_name, MemFdCreateFlag::MFD_CLOEXEC)
        .map(|fd| unsafe { File::from_raw_fd(fd) })
        .context("Failed to create an anomymous memory file")?;
    build
        .decompress_into(&mut file)
        .context("Failed to write the build to an anomymous memory file")?;

    let args = std::env::args_os()
        .map(|arg| CString::new(arg.into_vec()).map_err(Into::into))
        .collect::<Result<Vec<CString>>>()?;
    let env = std::env::vars_os()
        .map(|(mut key, value)| {
            key.push("=");
            key.push(value);

            CString::new(key.into_vec()).map_err(Into::into)
        })
        .collect::<Result<Vec<CString>>>()?;

    fexecve(file.into_raw_fd(), &args, &env).map_err(Into::into)
}
