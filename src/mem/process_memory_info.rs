use std::io::{Error, Result};

/// Process Memory Info returned by `get_process_memory_info`
#[derive(Debug, Clone, Default)]
pub struct ProcessMemoryInfo {
    /// this is the non-swapped physical memory a process has used.
    /// On UNIX it matches `top`'s RES column.
    ///
    /// On Windows this is an alias for wset field and it matches "Mem Usage"
    /// column of taskmgr.exe.
    pub resident_set_size: u64,

    pub resident_set_size_peak: Option<u64>,

    /// this is the total amount of virtual memory used by the process.
    /// On UNIX it matches `top`'s VIRT column.
    ///
    /// On Windows this is an alias for pagefile field and it matches "Mem
    /// Usage" "VM Size" column of taskmgr.exe.
    pub virtual_memory_size: u64,

    ///  This is the sum of:
    ///
    ///    + (internal - alternate_accounting)
    ///
    ///    + (internal_compressed - alternate_accounting_compressed)
    ///
    ///    + iokit_mapped
    ///
    ///    + purgeable_nonvolatile
    ///
    ///    + purgeable_nonvolatile_compressed
    ///
    ///    + page_table
    ///
    /// details: <https://github.com/apple/darwin-xnu/blob/master/osfmk/kern/task.c>
    pub phys_footprint: Option<u64>,

    pub compressed: Option<u64>,
}

#[cfg(target_os = "windows")]
fn get_process_memory_info_impl()
    -> anyhow::Result<ProcessMemoryInfo>
{
    use core::mem::MaybeUninit;
    use windows_sys::Win32::System::{
        ProcessStatus::{
            GetProcessMemoryInfo,
            PROCESS_MEMORY_COUNTERS,
        },
        Threading::GetCurrentProcess,
    };

    let mut process_memory_counters =
      MaybeUninit::<PROCESS_MEMORY_COUNTERS>::uninit();

    let sizeof_process_memory_counters =
        core::mem::size_of::<PROCESS_MEMORY_COUNTERS>();

    let ret = unsafe {
        // If the function succeeds, the return value is non-zero.
        // If the function fails, the return value is zero.
        // https://docs.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo

        GetProcessMemoryInfo(
            GetCurrentProcess(),
            process_memory_counters.as_mut_ptr(),
            sizeof_process_memory_counters as u32,
        )
    };

    if ret == 0 {
        return Err(Error::last_os_error());
    }

    let pmc =
        unsafe{ process_memory_counters.assume_init() };

    Ok(ProcessMemoryInfo {
        resident_set_size:
            pmc.WorkingSetSize as u64,

        resident_set_size_peak:
            Some(pmc.PeakWorkingSetSize as u64),

        virtual_memory_size:
            pmc.PagefileUsage as u64,

        phys_footprint: None,
        compressed: None,
    })
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_process_memory_info_impl()
    -> anyhow::Result<ProcessMemoryInfo>
{
    // https://www.kernel.org/doc/Documentation/filesystems/proc.txt

    use procfs::process::Process;
    let statm = Process::myself()?.statm()?;
    Ok(ProcessMemoryInfo {
        virtual_memory_size: statm.size,
        resident_set_size: statm.resident,

        resident_set_size_peak: None,
        phys_footprint: None,
        compressed: None,
    })
}

#[cfg(target_vendor="apple")]
fn get_process_memory_info_impl() -> Result<ProcessMemoryInfo> {
    //use crate::bindings::task_vm_info;

    use core::mem::MaybeUninit;

    use mach_sys::{
        kern_return::KERN_SUCCESS,
        message::mach_msg_type_number_t,
        task::task_info,
        task_info::{
            TASK_VM_INFO,
            task_vm_info_rev1_t,
            TASK_VM_INFO_REV1_COUNT
        },
        traps::mach_task_self,
        vm_types::natural_t,
    };

    let mut task_vm_info =
        MaybeUninit::<task_vm_info>::uninit();

    // https://github.com/apple/darwin-xnu/blob/master/osfmk/mach/task_info.h line 396
    // #define TASK_VM_INFO_COUNT	((mach_msg_type_number_t) \
    // (sizeof (task_vm_info_data_t) / sizeof (natural_t)))
    let mut task_info_cnt: mach_msg_type_number_t =
        (
            core::mem::size_of::<task_vm_info>()
            /
            core::mem::size_of::<natural_t>()
        ) as mach_msg_type_number_t;

    let kern_ret = unsafe {
        task_info(
            mach_task_self(),
            TASK_VM_INFO,
            task_vm_info.as_mut_ptr() as *mut _,
            &mut task_info_cnt,
        )
    };

    if kern_ret != KERN_SUCCESS {
        // see https://docs.rs/mach-sys/0.5.1/mach-sys/kern_return/index.html for more details
        anyhow::bail!(
            "DARWIN_KERN_RET_CODE: {}",
            kern_ret
        );
    }

    let task_vm_info =
        unsafe { task_vm_info.assume_init() };

    Ok(ProcessMemoryInfo {
        resident_set_size: task_vm_info.resident_size,
        virtual_memory_size: task_vm_info.virtual_size,

        resident_set_size_peak:
            Some(task_vm_info.resident_size_peak),
        phys_footprint:
            Some(task_vm_info.phys_footprint),
        compressed:
            Some(task_vm_info.compressed),
    })
}

pub fn get_process_memory_info() -> anyhow::Result<ProcessMemoryInfo> {
    get_process_memory_info_impl()
}
