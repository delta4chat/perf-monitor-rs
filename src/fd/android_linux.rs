use procfs::process::Process;

#[inline]
pub fn fd_count_pid<T: Into<u32>>(pid: T)
    -> anyhow::Result<usize>
{
    let pid: u32 = pid.into();

    let pid: i32 = pid.try_into()?;
    Ok(Process::new(pid)?.fd_count()?)
}

pub fn fd_count_current() -> anyhow::Result<usize> {
    Ok(Process::myself()?.fd_count()?)
}

#[deprecated]
pub fn fd_count_cur() -> std::io::Result<usize> {
    match fd_count_current() {
        Ok(v) => Ok(v),
        Err(e) => Err( std::io::Error::other(e) ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fd_count() {
        #[cfg(target_os = "linux")]
        const TEMP_DIR: &str = "/tmp";
        #[cfg(target_os = "android")]
        const TEMP_DIR: &str = "/data/local/tmp";

        const NUM: usize = 100;

        // open some files and do not close them.
        let fds: Vec<_> = (0..NUM)
            .map(|i| {
                let fname = format!("{}/tmpfile{}", TEMP_DIR, i);
                std::fs::File::create(fname).unwrap()
            })
            .collect();
        let count = fd_count_cur().unwrap();

        dbg!(count);
        assert!(count >= NUM);
        let old_count = count;

        drop(fds);
        let count = fd_count_cur().unwrap();
        // Though tests are run in multi-thread mode without using nextest, we
        // assume NUM is big enough to make fd count lower in a short period.
        assert!(count < old_count);
    }
}
