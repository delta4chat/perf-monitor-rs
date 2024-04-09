//! This crate provide the ability to retrieve information for profiling.
//!
//!

#![cfg_attr(test, allow(clippy::all, clippy::unwrap_used))]

#[cfg(test)]
extern crate test;

#[allow(warnings)]
#[cfg(all(feature="darwin_bindgen", target_vendor="apple"))]
pub(crate) mod darwin_bindings {
    include!(
        concat!(
            env!("OUT_DIR"),
            "/monitor_rs_ios_macos_binding.rs"
        )
    );
}

pub mod cpu;
pub use cpu::*;

pub mod mem;
pub use mem::*;

pub mod io;
pub use io::*;

pub mod fd;
pub use fd::*;

mod utils;
use utils::*;

