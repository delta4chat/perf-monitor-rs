use core::time::Duration;
use std::time::Instant;

use perfmon::cpu::processor_numbers;
use perfmon::cpu::ProcessStat;
use perfmon::cpu::ThreadStat;
use perfmon::fd::fd_count_cur;
use perfmon::io::get_process_io_stats;
use perfmon::mem::get_process_memory_info;

fn main() {
    build_some_threads();

    // cpu
    let core_num = processor_numbers().unwrap();
    let mut stat_p = ProcessStat::cur().unwrap();
    let mut stat_t = ThreadStat::cur().unwrap();

    let mut last_loop = Instant::now();
    loop {
        if last_loop.elapsed() > Duration::from_secs(1) {
            last_loop = Instant::now();
        } else {
            std::thread::sleep(Duration::from_micros(100));
            continue;
        }
        println!("----------");

        // cpu
        //let _ = (0..1_000).into_iter().sum::<i128>();

        let usage_p = stat_p.cpu().unwrap() * 100f64;
        let usage_t = stat_t.cpu().unwrap() * 100f64;

        println!(
            "[CPU] core Number: {}, process usage: {:.2}%, current thread usage: {:.2}%",
            core_num, usage_p, usage_t
        );

        // mem
        let mem_info = get_process_memory_info().unwrap();

        println!(
            "[Memory] memory used: {} bytes, virtural memory used: {} bytes ",
            mem_info.resident_set_size, mem_info.virtual_memory_size
        );

        // fd
        let fd_num = fd_count_cur().unwrap();

        println!("[FD] fd number: {}", fd_num);

        // io
        let io_stat = get_process_io_stats().unwrap();

        println!(
            "[IO] io-in: {} bytes, io-out: {} bytes",
            io_stat.read_bytes, io_stat.write_bytes
        );
    }
}

fn build_some_threads() {
    for i in 0..8 {
        std::thread::spawn(move || {
            let t_stat = ThreadStat::current().unwrap();
            loop {
                println!("{i} cpu usage: {:?}", t_stat.cpu());
                if fastrand::u8(..) % 4 == 0
                    && i%2 == 0 {
                    println!("{i} sleeping");
                    std::thread::sleep(
                        Duration::from_secs(1)
                    );
                    continue;
                }

                println!("{i} started");
                let _ = (0..(i as u128)+(u32::MAX / 400) as u128).into_iter().sum::<u128>();
                println!("{i} exited");
            }
        });
    }
}
