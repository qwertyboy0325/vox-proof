/// Process-level peak memory sampling for candidate-neutral measurements.
pub struct MemorySampler {
    baseline: u64,
    peak: u64,
}

impl MemorySampler {
    pub fn start() -> Self {
        let current = current_peak_bytes();
        Self {
            baseline: current,
            peak: current,
        }
    }

    pub fn observe(&mut self) {
        let current = current_peak_bytes();
        if current > self.peak {
            self.peak = current;
        }
    }

    pub fn peak_since_start(&self) -> u64 {
        self.peak.saturating_sub(self.baseline.min(self.peak))
    }
}

fn current_peak_bytes() -> u64 {
    #[cfg(unix)]
    {
        let mut usage = libc::rusage {
            ru_utime: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            ru_stime: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            ru_maxrss: 0,
            ru_ixrss: 0,
            ru_idrss: 0,
            ru_isrss: 0,
            ru_minflt: 0,
            ru_majflt: 0,
            ru_nswap: 0,
            ru_inblock: 0,
            ru_oublock: 0,
            ru_msgsnd: 0,
            ru_msgrcv: 0,
            ru_nsignals: 0,
            ru_nvcsw: 0,
            ru_nivcsw: 0,
        };
        let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
        if rc != 0 {
            return 0;
        }
        if cfg!(target_os = "macos") {
            usage.ru_maxrss as u64
        } else {
            usage.ru_maxrss as u64 * 1024
        }
    }
    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;
        use windows_sys::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        unsafe {
            let process = GetCurrentProcess();
            let mut counters = MaybeUninit::<PROCESS_MEMORY_COUNTERS>::uninit();
            let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok = GetProcessMemoryInfo(process, counters.as_mut_ptr(), size);
            if ok == 0 {
                return 0;
            }
            counters.assume_init().PeakWorkingSetSize as u64
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        0
    }
}

#[cfg(unix)]
mod libc {
    #[repr(C)]
    pub struct timeval {
        pub tv_sec: i64,
        pub tv_usec: i32,
    }

    #[repr(C)]
    pub struct rusage {
        pub ru_utime: timeval,
        pub ru_stime: timeval,
        pub ru_maxrss: i64,
        pub ru_ixrss: i64,
        pub ru_idrss: i64,
        pub ru_isrss: i64,
        pub ru_minflt: i64,
        pub ru_majflt: i64,
        pub ru_nswap: i64,
        pub ru_inblock: i64,
        pub ru_oublock: i64,
        pub ru_msgsnd: i64,
        pub ru_msgrcv: i64,
        pub ru_nsignals: i64,
        pub ru_nvcsw: i64,
        pub ru_nivcsw: i64,
    }

    pub const RUSAGE_SELF: i32 = 0;

    unsafe extern "C" {
        pub fn getrusage(who: i32, usage: *mut rusage) -> i32;
    }
}
