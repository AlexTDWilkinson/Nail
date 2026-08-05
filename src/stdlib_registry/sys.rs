//! Sys module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Sys:
        "sys_memory_total_bytes" [SysInfo] => "std_lib::sys::memory_total_bytes", () -> i,
            "Physical memory in bytes. format_bytes turns it into something readable.",
            "installed:i = sys_memory_total_bytes();";
        "sys_memory_available_bytes" [SysInfo] => "std_lib::sys::memory_available_bytes", () -> i,
            "Memory still available to programs, in bytes.",
            "headroom:i = sys_memory_available_bytes();";
        "sys_uptime_seconds" [SysInfo] => "std_lib::sys::uptime_seconds", () -> i,
            "Seconds since the machine booted.",
            "up_for:s = time_format_duration(sys_uptime_seconds());";
        "sys_load_average" [SysInfo] => "std_lib::sys::load_average", () -> f,
            "The one-minute load average - how many cores' worth of work is waiting. Above env_cpu_count means the machine is behind.",
            "load:f = sys_load_average();";
        "sys_cpu_usage_percent" [SysInfo, Tokio] => "std_lib::sys::cpu_usage_percent", () -> f,
            "CPU use across all cores as a percentage. Sampling takes a moment - the number is measured over a short interval, not read from a counter.",
            "busy:f = sys_cpu_usage_percent();";
        "sys_process_memory_bytes" [SysInfo] => "std_lib::sys::process_memory_bytes", () -> (i!e),
            "How much memory this very program is using, in bytes - the number to put on a health endpoint and watch for leaks.",
            "footprint:i = danger(sys_process_memory_bytes());";
        "sys_process_cpu_percent" [SysInfo, Tokio] => "std_lib::sys::process_cpu_percent", () -> (f!e),
            "How much CPU this very program is using, as a percentage of one core - 200.0 means two cores' worth. Sampled over a short interval.",
            "appetite:f = danger(sys_process_cpu_percent());";
        "sys_disk_free_bytes" [SysInfo] => "std_lib::sys::disk_free_bytes", (path: s) -> (i!e),
            "Bytes still free on the disk holding a path - the number that stops a full disk from being a surprise.",
            "free:i = danger(sys_disk_free_bytes(`/var/lib/my_app`));";
        "sys_disk_total_bytes" [SysInfo] => "std_lib::sys::disk_total_bytes", (path: s) -> (i!e),
            "The whole size of the disk holding a path, in bytes.",
            "size:i = danger(sys_disk_total_bytes(`/`));";
    }
}
