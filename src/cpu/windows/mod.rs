use super::processor_numbers;
use super::windows::process_times::ProcessTimes;
use super::windows::system_times::SystemTimes;
use super::windows::thread_times::ThreadTimes;

use core::time::Duration;
use core::cell::Cell;

use std::io::Result;

use windows_sys::Win32::{
    Foundation::FILETIME,
    System::Threading::GetCurrentThreadId,
};

pub mod process_times;
pub mod system_times;
pub mod thread_times;

#[derive(Debug, Copy, Clone)]
pub struct ThreadId(u32);

impl ThreadId {
    #[inline]
    pub fn current() -> Self {
        ThreadId(unsafe { GetCurrentThreadId() })
    }
}

/// convert to u64, unit 100 ns
fn filetime_to_ns100(ft: &FILETIME) -> u64 {
    /*
    let high = (ft.dwHighDateTime as u64) << 32);
    let low = ft.dwLowDateTime as u64;

    high + low
    */

    let high: [u8; 4] = ft.dwHighDateTime.to_be_bytes();
    let low: [u8; 4] = ft.dwLowDateTime.to_be_bytes();
    u64::from_be_bytes(high + low)
}

pub struct ThreadStat {
    tid: ThreadId,
    last_stat: Cell<(u64, u64)>,
}

impl ThreadStat {
    fn get_times(tid: ThreadId)
        -> Result<(u64, u64)>
    {
        let system_times =
            SystemTimes::capture()?;

        let thread_times =
            ThreadTimes::capture_with_thread_id(tid)?;

        let work_time =
            filetime_to_ns100(&thread_times.kernel)
            + filetime_to_ns100(&thread_times.user);

        let total_time =
            filetime_to_ns100(&system_times.kernel)
            + filetime_to_ns100(&system_times.user);

        Ok( (work_time, total_time) )
    }

    pub fn current() -> Result<Self> {
        let tid = ThreadId::current();
        let times = Self::get_times(tid)?;
        Ok(ThreadStat {
            tid,
            last_stat: Cell::new(times)
        })
    }

    pub fn build(tid: ThreadId) -> Result<Self> {
        let times = Self::get_times(tid)?;
        Ok(ThreadStat {
            tid,
            last_stat: Cell::new(times),
        })
    }

    pub fn cpu(&self) -> Result<f64> {
        let (work_time, total_time) =
            Self::get_times(self.tid)?;

        let (old_work_time, old_total_time) =
            self.last_stat.replace(
                (work_time, total_time)
            );

        let dt_total_time = total_time - old_total_time;

        if dt_total_time == 0 {
            return Ok(0.0);
        }

        let dt_work_time = work_time - old_work_time;

        let cpus = processor_numbers()?;
        Ok(
            (dt_work_time as f64)/(dt_total_time as f64)
            * (cpus as f64)
        )
    }

    pub fn cpu_time(&self) -> Result<Duration> {
        let (work_time, total_time) =
            Self::get_times(self.tid)?;

        let (old_work_time, old_total_time) =
            self.last_stat.replace(
                (work_time, total_time)
            );

        let cpu_time = work_time - old_work_time;

        Ok(
            Duration::from_nanos(cpu_time)
        )
    }
}

#[inline]
pub fn cpu_time() -> Result<Duration> {
    let process_times =
        ProcessTimes::capture_current()?;

    let kt = filetime_to_ns100(&process_times.kernel);
    let ut = filetime_to_ns100(&process_times.user);

    // convert ns
    //
    // Note: make it ns unit may overflow in some cases.
    // For example, a machine with 128 cores runs for one year.
    let mut cpu_time = (kt + ut).saturating_mul(100);

    // make it un-normalized
    let cpus = processor_numbers()?;
    let cpu_time *= (cpus as u64);

    Ok( Duration::from_nanos(cpu_time) )
}

