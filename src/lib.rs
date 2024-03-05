//! This crate provide the ability to retrieve information for profiling.
//!
//!

#![cfg_attr(test, allow(clippy::all, clippy::unwrap_used))]
#![cfg_attr(doc, feature(doc_cfg))]
#![cfg_attr(test, feature(test))]

#[cfg(test)]
extern crate test;

#[allow(warnings)]
#[cfg(target_vendor="apple")]
pub(crate) mod bindings {
    pub use mach_sys::*;
}
/*
pub(crate) mod bindings {
    include!(
        concat!(
            env!("OUT_DIR"),
            "/monitor_rs_ios_macos_binding.rs"
        )
    );
}*/

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

