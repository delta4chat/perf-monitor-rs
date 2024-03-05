
use core::convert::TryInto;
use core::cell::Cell;
use core::time::Duration;

use std::time::Instant;

use procfs::process::{Process, Task, Stat};
use procfs::{CpuInfo, ticks_per_second};

use once_cell::sync::Lazy;

pub static TICKS_PER_SECOND: Lazy<u64> =
    Lazy::new(ticks_per_second);

pub static CPU_INFO: Lazy<CpuInfo> =
    Lazy::new(|| {
        use procfs::Current;
        CpuInfo::current()
            .expect("cannot get /proc/cpuinfo")
    });

pub fn current_process() -> anyhow::Result<Process> {
    Ok(Process::myself()?)
}
pub fn current_task() -> anyhow::Result<Task> {
    let tid: i32 = ThreadId::current().into();
    Ok(current_process()?.task_from_tid(tid)?)
}

#[derive(Debug, Copy, Clone)]
pub struct ThreadId(rustix::thread::Pid);

impl From<ThreadId> for i32 {
    fn from(val: ThreadId) -> i32 {
        val.0.as_raw_nonzero().get()
    }
}
impl ThreadId {
    #[inline]
    pub fn current() -> Self {
        ThreadId( rustix::thread::gettid() )
    }
}

fn get_thread_stat(tid: ThreadId)
    -> anyhow::Result<Stat>
{
    let task =
        current_process()?
        .task_from_tid( tid.into() )?;

    let stat = task.stat()?;
    Ok(stat)
}

fn get_thread_cputime(tid: ThreadId)
    -> anyhow::Result<Duration>
{
    let stat = get_thread_stat(tid)?;
    get_stat_cputime(stat)
}

enum Ticks {
    U64(u64),
    I64(i64),
}

impl From<u64> for Ticks {
    fn from(val: u64) -> Ticks {
        Ticks::U64(val)
    }
}
impl From<i64> for Ticks {
    fn from(val: i64) -> Ticks {
        Ticks::I64(val)
    }
}

impl From<Ticks> for f64 {
    fn from(val: Ticks) -> f64 {
        use Ticks::*;
        match val {
            U64(u) => { u as f64 }
            I64(i) => { i as f64 }
        }
    }
}

fn ticks_to_seconds<T: Into<Ticks>>(ticks: T)
    -> anyhow::Result<f64>
{
    let ticks: Ticks = ticks.into();
    let ticks: f64 = ticks.into();

    let tps: u64 = *TICKS_PER_SECOND;
    if tps == 0 {
        anyhow::bail!("unexpected zero value of TICKS_PER_SECOND");
    }

    Ok(  ticks / (tps as f64)  )
}
fn get_stat_cputime(stat: Stat)
    -> anyhow::Result<Duration>
{
    let utime = ticks_to_seconds(stat.utime)?;
    let stime = ticks_to_seconds(stat.stime)?;
    let cutime = ticks_to_seconds(stat.cutime)?;
    let cstime = ticks_to_seconds(stat.cstime)?;

    let total_cputime = utime + stime + cutime + cstime;
    if total_cputime < 0.0 {
        anyhow::bail!(
            "cputime({}) should not a negative number!",
            total_cputime,
        );
    }

    Ok(Duration::from_secs_f64( total_cputime.abs() ))
}

pub struct ThreadStat {
    tid: ThreadId,
    last_stat: Cell<(Duration, Instant)>,
}

impl TryFrom<ThreadId> for ThreadStat {
    type Error = anyhow::Error;

    fn try_from(tid: ThreadId)
        -> anyhow::Result<ThreadStat>
    {
        let cputime = get_thread_cputime(tid)?;
        let total_time = Instant::now();
        Ok(ThreadStat {
            tid,
            last_stat: Cell::new((cputime, total_time)),
        })
    }
}
impl ThreadStat {
    pub fn current() -> anyhow::Result<Self> {
        ThreadId::current().try_into()
    }

    #[deprecated]
    pub fn cur() -> std::io::Result<Self> {
        match Self::current() {
            Ok(v) => Ok(v),
            Err(e) => Err( std::io::Error::other(e) ),
        }
    }

    #[deprecated]
    pub fn build(tid: ThreadId)-> std::io::Result<Self>{
        match tid.try_into() {
            Ok(v) => Ok(v),
            Err(e) => Err( std::io::Error::other(e) ),
        }
    }

    /// un-normalized
    pub fn cpu_usage(&self) -> anyhow::Result<f64> {
        let cputime = get_thread_cputime(self.tid)?;
        let total_time = Instant::now();

        let (old_cputime, old_total_time) =
            self.last_stat.replace(
                (cputime, total_time)
            );


        let dt_cputime_f64: f64 =
            if cputime >= old_cputime {
                (cputime - old_cputime)
                .as_secs_f64()
            } else {
                let t =
                    (old_cputime - cputime)
                    .as_secs_f64();
                -t
            };

        let dt_total_time_f64: f64 =
            total_time
            .saturating_duration_since(old_total_time)
            .as_secs_f64();

        Ok(dt_cputime_f64 / dt_total_time_f64)
    }

    #[deprecated]
    pub fn cpu(&self) -> std::io::Result<f64> {
        match self.cpu_usage() {
            Ok(v) => Ok(v),
            Err(e) => Err( std::io::Error::other(e) ),
        }
    }

    pub fn cpu_time(&self) -> anyhow::Result<Duration> {
        let cputime = get_thread_cputime(self.tid)?;
        let total_time = Instant::now();
        let (old_cputime, _old_total_time) =
            self.last_stat.replace(
                (cputime, total_time)
            );

        Ok( cputime.saturating_sub(old_cputime) )
    }
}

/// get cpu time of provided PID.
pub fn process_cputime<T: Into<i32>>(pid: T)
    -> anyhow::Result<Duration>
{
    let stat = Process::new(pid.into())?.stat()?;
    get_stat_cputime(stat)
}

/// get cpu time of current process.
pub fn cpu_time() -> anyhow::Result<Duration> {
    let stat = current_process()?.stat()?;
    get_stat_cputime(stat)
}

