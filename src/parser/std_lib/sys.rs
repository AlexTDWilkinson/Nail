//! What the machine looks like from inside - memory, load, disks, uptime.
//! The numbers a health endpoint reports and an ops dashboard charts.

/// Physical memory in bytes.
pub fn memory_total_bytes() -> i64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    return system.total_memory() as i64;
}

/// Memory still available to programs, in bytes.
pub fn memory_available_bytes() -> i64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    return system.available_memory() as i64;
}

/// Seconds since the machine booted.
pub fn uptime_seconds() -> i64 {
    return sysinfo::System::uptime() as i64;
}

/// The one-minute load average - how many cores' worth of work is waiting.
pub fn load_average() -> f64 {
    return sysinfo::System::load_average().one;
}

/// CPU use across all cores as a percentage. Sampling takes a moment - the
/// number is measured over a short interval, not read from a counter.
pub async fn cpu_usage_percent() -> f64 {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_usage();
    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
    system.refresh_cpu_usage();
    return system.global_cpu_usage() as f64;
}

/// How much memory this very program is using, in bytes - the number to put
/// on a health endpoint and watch for leaks.
pub fn process_memory_bytes() -> Result<i64, String> {
    let pid = sysinfo::get_current_pid().map_err(|e| format!("sys_process_memory_bytes: could not tell which process this is: {}", e))?;
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]));
    let process = system.process(pid).ok_or_else(|| "sys_process_memory_bytes: this process is not in the process table".to_string())?;
    return Ok(process.memory() as i64);
}

/// How much CPU this very program is using, as a percentage of one core -
/// 200.0 means two cores' worth. Sampled over a short interval, like
/// sys_cpu_usage_percent.
pub async fn process_cpu_percent() -> Result<f64, String> {
    let pid = sysinfo::get_current_pid().map_err(|e| format!("sys_process_cpu_percent: could not tell which process this is: {}", e))?;
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]));
    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]));
    let process = system.process(pid).ok_or_else(|| "sys_process_cpu_percent: this process is not in the process table".to_string())?;
    return Ok(process.cpu_usage() as f64);
}

fn disk_holding(path: &str, what: &str) -> Result<(std::path::PathBuf, sysinfo::Disks), String> {
    let target = std::fs::canonicalize(path).map_err(|e| format!("{}: `{}` does not exist: {}", what, path, e))?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    return Ok((target, disks));
}

/// Bytes still free on the disk holding a path.
pub fn disk_free_bytes(path: String) -> Result<i64, String> {
    let (target, disks) = disk_holding(&path, "sys_disk_free_bytes")?;
    let best = disks
        .iter()
        .filter(|disk| target.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .ok_or_else(|| format!("sys_disk_free_bytes: no disk holds `{}`", path))?;
    return Ok(best.available_space() as i64);
}

/// The whole size of the disk holding a path, in bytes.
pub fn disk_total_bytes(path: String) -> Result<i64, String> {
    let (target, disks) = disk_holding(&path, "sys_disk_total_bytes")?;
    let best = disks
        .iter()
        .filter(|disk| target.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .ok_or_else(|| format!("sys_disk_total_bytes: no disk holds `{}`", path))?;
    return Ok(best.total_space() as i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_has_memory_and_an_uptime() {
        assert!(memory_total_bytes() > 0);
        let available = memory_available_bytes();
        assert!(available > 0 && available <= memory_total_bytes());
        assert!(uptime_seconds() > 0);
        assert!(load_average() >= 0.0);
    }

    #[tokio::test]
    async fn cpu_use_is_a_percentage() {
        let usage = cpu_usage_percent().await;
        assert!((0.0..=100.0).contains(&usage), "got {}", usage);
    }

    #[tokio::test]
    async fn the_program_knows_its_own_appetite() {
        let memory = process_memory_bytes().unwrap();
        assert!(memory > 0, "got {}", memory);
        let cpu = process_cpu_percent().await.unwrap();
        assert!(cpu >= 0.0, "got {}", cpu);
    }

    #[test]
    fn the_root_disk_exists_and_a_fake_path_does_not() {
        let free = disk_free_bytes("/".to_string()).unwrap();
        let total = disk_total_bytes("/".to_string()).unwrap();
        assert!(free > 0 && free <= total);
        assert!(disk_free_bytes("/no/such/path/anywhere".to_string()).unwrap_err().contains("does not exist"));
    }
}
