use core_foundation::base::CFRelease;
use libc::{c_int, c_uint};
use mach2::{
    host_priv::mach_host_self,
    kern_return::KERN_SUCCESS,
    mach_host::host_statistics64,
    message::mach_msg_type_number_t,
    processor_info::{host_processor_info, processor_cpu_load_info_data_t, PROCESSOR_CPU_LOAD_INFO},
    vm_statistics::{vm_statistics64, VM_STATISTICS64_COUNT},
};
use std::mem::size_of;
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Metrics {
    pub cpu_percent: f32,
    pub mem_used_gb: f32,
    pub mem_total_gb: f32,
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
}

pub fn sample() -> Metrics {
    let (cpu_percent, _) = cpu_percent();
    let (mem_used_gb, mem_total_gb) = mem_gb();
    Metrics {
        cpu_percent,
        mem_used_gb,
        mem_total_gb,
        cpu_temp_c: None, // fill using IOReport later
        gpu_temp_c: None, // fill using IOReport later
    }
}

/* ---------------- CPU usage % ---------------- */

lazy_static::lazy_static! {
    static ref LAST_CPU: Mutex<Option<Vec<[u64; 4]>>> = Mutex::new(None);
}

pub fn cpu_percent() -> (f32, usize) {
    unsafe {
        let host = mach_host_self();
        let mut count: mach_msg_type_number_t = 0;
        let mut cpu_info: *mut processor_cpu_load_info_data_t = std::ptr::null_mut();
        let mut cpu_count: c_uint = 0;

        let ret = host_processor_info(
            host,
            PROCESSOR_CPU_LOAD_INFO,
            &mut cpu_count,
            &mut (cpu_info as *mut _),
            &mut count,
        );

        if ret != KERN_SUCCESS {
            return (0.0, 0);
        }

        let per_cpu = std::slice::from_raw_parts(cpu_info, cpu_count as usize);
        let mut new = Vec::with_capacity(cpu_count as usize);

        for cpu in per_cpu {
            let user = cpu.cpu_ticks[0] as u64;
            let nice = cpu.cpu_ticks[1] as u64;
            let system = cpu.cpu_ticks[2] as u64;
            let idle = cpu.cpu_ticks[3] as u64;
            new.push([user, nice, system, idle]);;
        }

        let mut last_guard = LAST_CPU.lock().unwrap();
        let (percent, ncpu) = if let Some(last) = &*last_guard {
            let mut used: u64 = 0;
            let mut total: u64 = 0;

            for (i, cur) in new.iter().enumerate() {
                let prev = last[i];
                let du = cur[0] as i64 - prev[0] as i64;
                let dn = cur[1] as i64 - prev[1] as i64;
                let ds = cur[2] as i64 - prev[2] as i64;
                let di = cur[3] as i64 - prev[3] as i64;
                let dused = (du + dn + ds).max(0) as u64;
                let dtotal = (du + dn + ds + di).max(0) as u64;
                used += dused;
                total += dtotal;
            }
            let percent = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            (percent, new.len())
        } else {
            (0.0, new.len())
        };

        *last_guard = Some(new);

        let bytes = (count as usize) * std::mem::size_of::<u32>();
        let _ = vm_deallocate(
            mach_task_self(),
            cpu_info as vm_address_t,
            bytes as vm_size_t,
        );

        (percent, ncpu)
    }
}


/* ---------------- Memory GB ---------------- */

pub fn mem_gb() -> (f32, f32) {
    unsafe {
        let host = mach_host_self();
        let mut count = VM_STATISTICS64_COUNT;
        let mut vmstat: vm_statistics64 = std::mem::zeroed();

        let ret = host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            (&mut vmstat as *mut vm_statistics64) as *mut i32,
            &mut count,
        );
        if ret != KERN_SUCCESS {
            return (0.0, 0.0);
        }

        // Total physical
        let mut size: libc::size_t = std::mem::size_of::<u64>();
        let mut memsize: u64 = 0;
        let name = std::ffi::CString::new("hw.memsize").unwrap();
        let r = libc::sysctlbyname(
            name.as_ptr(),
            &mut memsize as *mut u64 as *mut _,
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if r != 0 {
            return (0.0, 0.0);
        }

        let page_size: u64 = libc::sysconf(libc::_SC_PAGESIZE) as u64; // or host_page_size

        // active + wired ~ “used”
        let used = (vmstat.active_count + vmstat.wire_count) as u64 * page_size;
        let total = memsize;

        (bytes_to_gb(used), bytes_to_gb(total))
    }
}

fn bytes_to_gb(b: u64) -> f32 {
    (b as f64 / (1024.0 * 1024.0 * 1024.0)) as f32
}
