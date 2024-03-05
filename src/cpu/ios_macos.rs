use libc::{
    // functions
    mach_thread_self, thread_info,

    // structs, types, and constants
    rusage, RUSAGE_SELF,

    time_value_t, timeval,

    KERN_SUCCESS,

    thread_basic_info_t,
    THREAD_BASIC_INFO, THREAD_BASIC_INFO_COUNT,
};

use core::convert::TryInto;
use core::mem::MaybeUninit;
use core::time::Duration;

use std::time::Instant;

#[derive(Debug, Copy, Clone)]
pub struct ThreadId(u32);

impl From<ThreadId> for u32 {
    fn from(val: ThreadId) -> u32 {
        val.0
    }
}
impl ThreadId {
    #[inline]
    pub fn current() -> Self {
        ThreadId(unsafe { mach_thread_self() })
    }
}

fn get_thread_basic_info(tid: ThreadId)
    -> anyhow::Result<thread_basic_info_t>
{
    let mut basic_info =
        MaybeUninit::<thread_basic_info_t>::uninit();

    let mut basic_info_cnt =
        THREAD_BASIC_INFO_COUNT;

    let ret = unsafe {
        thread_info(
            tid.into(),
            THREAD_BASIC_INFO as u32,
            basic_info.as_mut_ptr() as *mut _,
            &mut basic_info_cnt,
        )
    };

    if ret != (KERN_SUCCESS as i32) {
        return Err( Error::from_raw_os_error(ret) );
    }
    Ok(unsafe { basic_info.assume_init() })
}

#[derive(Debug, Clone)]
pub struct ThreadStat {
    tid: ThreadId,
    last_stat: Cell<(thread_basic_info_t, Instant)>,
}

impl TryFrom<ThreadId> for ThreadStat {
    type Error = anyhow::Error;
    fn try_from(tid: ThreadId)
        -> anyhow::Result<Self>
    {
        let stat = get_thread_basic_info(tid)?;
        let time = Instant::now();
        Ok(ThreadStat {
            tid,
            last_stat: Cell::new( (stat, time) ),
        })
    }
}

impl ThreadStat {
    pub fn current() -> anyhow::Result<Self> {
        Self::build(ThreadId::current())
    }

    #[deprecated]
    pub fn cur() -> std::io::Result<Self> {
        Self::current()
    }

    #[deprecated]
    pub fn build(tid: ThreadId)
        -> std::io::Result<Self>
    {
        tid.try_into()
    }

    /// un-normalized
    pub fn cpu_usage(&self) -> anyhow::Result<f64> {
        let stat = get_thread_basic_info(self.tid)?;
        let now = Instant::now();

        let (old_stat, old_now) =
            self.last_stat.replace( (stat, now) );

        let utime =
          time_value_to_duration(stat.user_time);
        let stime =
          time_value_to_duration(stat.system_time);

        let old_utime =
          time_value_to_duration(old_stat.user_time);
        let old_stime =
          time_value_to_duration(old_stat.system_time);

        let dt_utime = utime.saturating_sub(old_utime);
        let dt_stime = stime.saturating_sub(old_stime);

        let dt_cputime_micros: u128 =
            dt_utime.saturating_add(dt_stime)
                    .as_micros();

        let mut dt_duration_micros: u128 =
            now.saturating_duration_since(old_now)
                .as_micros();
        if dt_duration_micros == 0 {
            // this avoids "division by zero"
            dt_duration_micros = 1;
        }

        Ok(
            (dt_cputime_micros as f64)
            /
            (dt_duration_micros as f64)
        )
    }

    #[deprecated]
    pub fn cpu(&self) -> std::io::Result<f64> {
        self.cpu_usage()
    }

    pub fn cpu_time(&self) -> anyhow::Result<Duration> {
        let stat = get_thread_basic_info(self.tid)?;
        let now = Instant::now();

        let (old_stat, _old_now) =
            self.last_stat.replace( (stat, now) );

        let utime =
          time_value_to_duration(stat.user_time);
        let stime =
          time_value_to_duration(stat.system_time);

        let old_utime =
          time_value_to_duration(old_stat.user_time);
        let old_stime =
          time_value_to_duration(old_stat.system_time);
        
        let dt_utime = utime.saturating_sub(old_utime);
        let dt_stime = stime.saturating_sub(old_stime);

        let dt_cputime: Duration =
            dt_utime.saturating_add(dt_stime);

        Ok(dt_cputime)
    }
}

#[inline]
fn time_value_to_duration(t: time_value_t) -> Duration {
    let secs: Duration =
        Duration::from_secs(
            t.seconds.try_into().unwrap_or(0)
        );

    let sub_secs: Duration =
        Duration::from_micros(
            t.microseconds.try_into().unwrap_or(0)
        );

    secs.saturating_add(sub_secs)
}

#[inline]
fn timeval_to_duration(t: timeval) -> Duration {
    let secs: Duration =
        Duration::from_secs(
            t.tv_sec.try_into().unwrap_or(0)
        );

    let sub_secs: Duration =
        Duration::from_nanos(
            t.tv_nsec.try_into().unwrap_or(0)
        );

    secs.saturating_add(sub_secs)
}

#[deprecated]
fn time_value_to_u64(tv: time_value_t) -> u64 {
    time_value_to_duration(tv).as_micros() as u64
}

pub fn cpu_time() -> anyhow::Result<Duration> {
    let mut time = MaybeUninit::<rusage>::uninit();
    let ret =
        unsafe {
            libc::getrusage(
                RUSAGE_SELF,
                time.as_mut_ptr()
            )
        };

    if ret != 0 {
        return Err(Error::last_os_error());
    }

    let time = unsafe { time.assume_init() };

    let sec =
        (time.ru_utime.tv_sec as u64)
        .saturating_add(time.ru_stime.tv_sec as u64);
    let nsec =
        (time.ru_utime.tv_usec as u32)
        .saturating_add(time.ru_stime.tv_usec as u32)
        .saturating_mul(1000);
    Ok(Duration::new(sec, nsec))
}

#[cfg(test)]
#[allow(clippy::all, clippy::print_stdout)]
mod tests {
    use super::*;
    use test::Bencher;

    // There is a field named `cpu_usage` in `thread_basic_info` which represents the CPU usage of the thread.
    // However, we have no idea about how long the interval is. And it will make the API being different from other platforms.
    // We calculate the usage instead of using the field directory to make the API is the same on all platforms.
    // The cost of the calculation is very very small according to the result of the following benchmark.
    #[bench]
    fn bench_cpu_usage_by_calculate(b: &mut Bencher) {
        let tid = ThreadId::current();
        let last_stat = get_thread_basic_info(tid).unwrap();
        let last_time = Instant::now();

        b.iter(|| {
            let cur_stat = get_thread_basic_info(tid).unwrap();
            let cur_time = Instant::now();

            let cur_user_time = time_value_to_u64(cur_stat.user_time);
            let cur_sys_time = time_value_to_u64(cur_stat.system_time);
            let last_user_time = time_value_to_u64(last_stat.user_time);
            let last_sys_time = time_value_to_u64(last_stat.system_time);

            let dt_duration = cur_time - last_time;
            let cpu_time_us = cur_user_time + cur_sys_time - last_user_time - last_sys_time;
            let dt_wtime = Duration::from_micros(cpu_time_us);

            let _ = (cur_stat, cur_time);
            let _ = dt_wtime.as_micros() as f64 / dt_duration.as_micros() as f64;
        });
    }

    #[bench]
    fn bench_cpu_usage_by_field(b: &mut Bencher) {
        let tid = ThreadId::current();
        b.iter(|| {
            let cur_stat = get_thread_basic_info(tid).unwrap();
            let _ = cur_stat.cpu_usage / 1000;
        });
    }
}
