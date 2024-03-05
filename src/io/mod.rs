//! Get io usage for current process.

/*
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("IOStatsError({code}):{msg}")]
pub struct IOStatsError {
    pub code: i32,
    pub msg: String,
}

impl From<std::io::Error> for IOStatsError {
    fn from(e: std::io::Error) -> Self {
        Self {
            code: e.kind() as i32,
            msg: e.to_string(),
        }
    }
}

impl From<std::num::ParseIntError> for IOStatsError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self {
            code: 0,
            msg: e.to_string(),
        }
    }
}
*/

/// A struct represents io status.
#[derive(Debug, Clone, Default)]
pub struct IOStats {
    /// (linux & windows)
    /// the number of read operations performed (cumulative)
    pub read_count: Option<u64>,

    /// (linux & windows)
    /// the number of write operations performed (cumulative)
    pub write_count: Option<u64>,

    /// (all supported platforms)
    /// the number of bytes read (cumulative).
    pub read_bytes: u64,

    /// (all supported platforms)
    /// the number of bytes written (cumulative)
    pub write_bytes: u64,
}

/// Get the io stats of current process. Most platforms are supported.
///
/// in any platforms that is not supported, this function will always returns error.
pub fn get_process_io_stats()
    -> anyhow::Result<IOStats>
{
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "windows"
    ))]
    {
        return get_process_io_stats_impl();
    }

    anyhow::bail!("cannot get I/O stats: this platform is not supported");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_process_io_stats_impl()
    -> anyhow::Result<IOStats>
{
    use procfs::process::Process;
    let ret = Process::myself()?.io()?;
    Ok(IOStats {
        read_count: Some(ret.syscr),
        write_count: Some(ret.syscw),

        // NOTE we just need real disk I/O to be calculated, so do not use ".rchar" or ".wchar"
        read_bytes: ret.read_bytes,
        write_bytes: ret.write_bytes,
    })
}

#[cfg(target_os = "windows")]
fn get_process_io_stats_impl()
    -> anyhow::Result<IOStats>
{
    use core::mem::MaybeUninit;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess,
        GetProcessIoCounters,
        IO_COUNTERS,
    };

    let mut io_counters =
        MaybeUninit::<IO_COUNTERS>::uninit();

    let ret = unsafe {
        // If the function succeeds, the return value is nonzero.
        // If the function fails, the return value is zero.
        // https://docs.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getprocessiocounters

        GetProcessIoCounters(
            GetCurrentProcess(),
            io_counters.as_mut_ptr(),
        )
    };

    if ret == 0 {
        return Err(
            std::io::Error::last_os_error().into()
        );
    }

    let ic = unsafe { io_counters.assume_init() };

    Ok(IOStats {
        read_count: Some(ic.ReadOperationCount),
        write_count: Some(ic.WriteOperationCount),

        read_bytes: ic.ReadTransferCount,
        write_bytes: ic.WriteTransferCount,
    })
}

#[cfg(target_os = "macos")]
fn get_process_io_stats_impl()
    -> anyhow::Result<IOStats>
{
    use libc::{rusage_info_v2, RUSAGE_INFO_V2};
    use core::{mem::MaybeUninit, ffi::c_int};

    let mut rusage_info_v2 =
        MaybeUninit::<rusage_info_v2>::uninit();

    let ret_code = unsafe {
        libc::proc_pid_rusage(
            std::process::id() as c_int,
            RUSAGE_INFO_V2,
            rusage_info_v2.as_mut_ptr() as *mut _,
        )
    };

    if ret_code != 0 {
        return Err(
            std::io::Error::last_os_error().into()
        );
    }

    let ri_v2 = unsafe { rusage_info_v2.assume_init() };

    Ok(IOStats {
        read_count: None,
        write_count: None,

        read_bytes: ri_v2.ri_diskio_bytesread,
        write_bytes: ri_v2.ri_diskio_byteswritten,
    })
}

