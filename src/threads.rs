//! How many threads a compiled Nail program runs on, decided once when it
//! starts, from the machine it started on. Nobody writing Nail has to know
//! what a thread pool is, and nothing has to be set.
//!
//! The rule: the physical cores of the NUMA node the program woke up on, and
//! the program is pinned to that node. On a laptop or a droplet that is
//! simply the physical cores. On a four-socket machine it is one socket.
//!
//! Why one socket. Take a four-socket server with 96 cores and 192 threads
//! across four NUMA nodes. Rayon's default pool is every logical CPU, so a
//! fork-join over a few thousand elements is split 192 ways, shipped to cores
//! whose memory is three hops away, and joined with a 192-way barrier. When
//! the work per piece is microseconds, the barrier costs more than the work,
//! and a job like that measured ten times slower per unit than the same code
//! on a 24-core desktop. Giving it one socket's worth of threads was the
//! whole 10x. A collection operation with a cheap body is exactly that
//! shape, and it is what every Nail program emits, so the default has to be
//! the one that works on that machine. On a machine with one socket it
//! changes nothing but the SMT threads.
//!
//! Why physical cores and not SMT threads. A sibling thread shares its core's
//! execution units, so for compute it adds a little throughput and a lot of
//! contention on the joins. The joins are what were slow.
//!
//! `NAIL_THREADS` (or rayon's own `RAYON_NUM_THREADS`) overrides the count
//! for the rare job that wants the whole box, and then nothing is pinned.
//! It is not documented for Nail programmers: the point is that they never
//! need it.
//!
//! The tokio runtime the transpiler builds around `main` uses the same number
//! for its workers, so a program's threads are one story, not two.

use std::collections::BTreeSet;

/// Decides the thread count, pins the process when that is part of the plan,
/// sizes the global rayon pool, and returns the count for the tokio runtime
/// to use. Called first thing in every generated `main`.
pub fn configure() -> usize {
    let plan = plan();
    #[cfg(target_os = "linux")]
    if let Some(cpus) = &plan.pin_to {
        linux::pin_to(cpus);
    }
    // Already built means something configured it before us, which is fine:
    // the count we return still sizes the tokio runtime.
    let _ = rayon::ThreadPoolBuilder::new().num_threads(plan.threads).build_global();
    plan.threads
}

/// What the program will do: how many threads, and which CPUs to stay on.
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub threads: usize,
    pub pin_to: Option<BTreeSet<usize>>,
}

fn plan() -> Plan {
    if let Some(asked) = requested_count() {
        return Plan { threads: asked, pin_to: None };
    }
    #[cfg(target_os = "linux")]
    if let Some(plan) = linux::plan() {
        return plan;
    }
    Plan { threads: std::thread::available_parallelism().map(|count| count.get()).unwrap_or(1), pin_to: None }
}

/// An explicit count from the environment, when someone gave one.
fn requested_count() -> Option<usize> {
    ["NAIL_THREADS", "RAYON_NUM_THREADS"].iter().find_map(|name| std::env::var(name).ok()).and_then(|value| value.trim().parse::<usize>().ok()).filter(|count| *count > 0)
}

/// The policy, separated from the machine so it can be tested against any
/// machine: given the CPU the program is on, the CPU sets of the NUMA nodes,
/// the CPUs the process is allowed to use, and the (package, core) pair each
/// CPU belongs to, which CPUs should it run on and how many threads is that.
pub fn choose(current_cpu: usize, nodes: &[BTreeSet<usize>], allowed: &BTreeSet<usize>, core_of: impl Fn(usize) -> Option<(usize, usize)>) -> Plan {
    let home = nodes.iter().find(|node| node.contains(&current_cpu)).cloned().unwrap_or_else(|| allowed.clone());
    let mut usable: BTreeSet<usize> = home.intersection(allowed).cloned().collect();
    if usable.is_empty() {
        usable = allowed.clone();
    }
    let threads = physical_cores(&usable, core_of).max(1);
    // Pinning only matters when there is more than one node and the plan
    // leaves some allowed CPUs out. A single-node machine, or a process
    // already confined to one node by taskset or a cgroup, is left alone.
    let pin_to = if nodes.len() > 1 && usable.len() < allowed.len() { Some(usable) } else { None };
    Plan { threads, pin_to }
}

/// Distinct physical cores among a set of CPUs. A CPU whose core cannot be
/// read counts as its own core, so a stripped container degrades to logical
/// CPUs rather than to nothing.
pub fn physical_cores(cpus: &BTreeSet<usize>, core_of: impl Fn(usize) -> Option<(usize, usize)>) -> usize {
    let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut unknown = 0;
    for cpu in cpus {
        match core_of(*cpu) {
            Some(core) => {
                seen.insert(core);
            }
            None => unknown += 1,
        }
    }
    seen.len() + unknown
}

/// The kernel's CPU list syntax: `0-3,8,10-11`, with a trailing newline.
pub fn parse_cpu_list(text: &str) -> BTreeSet<usize> {
    let mut cpus = BTreeSet::new();
    for part in text.trim().split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((start, end)) => {
                if let (Ok(start), Ok(end)) = (start.trim().parse::<usize>(), end.trim().parse::<usize>()) {
                    cpus.extend(start..=end);
                }
            }
            None => {
                if let Ok(cpu) = part.parse::<usize>() {
                    cpus.insert(cpu);
                }
            }
        }
    }
    cpus
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{choose, parse_cpu_list, Plan};
    use std::collections::BTreeSet;

    pub fn plan() -> Option<Plan> {
        let allowed = allowed_cpus()?;
        let current = current_cpu()?;
        let nodes = numa_nodes();
        Some(choose(current, &nodes, &allowed, core_of))
    }

    /// The CPUs this process may run on, as taskset or a cgroup left them.
    fn allowed_cpus() -> Option<BTreeSet<usize>> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|line| line.starts_with("Cpus_allowed_list:"))?;
        let cpus = parse_cpu_list(line.trim_start_matches("Cpus_allowed_list:"));
        if cpus.is_empty() {
            None
        } else {
            Some(cpus)
        }
    }

    fn current_cpu() -> Option<usize> {
        // Safe: sched_getcpu takes no arguments and only reads scheduler state.
        let cpu = unsafe { libc::sched_getcpu() };
        if cpu < 0 {
            None
        } else {
            Some(cpu as usize)
        }
    }

    /// Each NUMA node's CPUs. Empty when the machine does not say, which
    /// `choose` treats as one node made of everything allowed.
    fn numa_nodes() -> Vec<BTreeSet<usize>> {
        let mut nodes = Vec::new();
        let Ok(entries) = std::fs::read_dir("/sys/devices/system/node") else { return nodes };
        let mut names: Vec<String> = entries.flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).filter(|name| name.starts_with("node") && name[4..].chars().all(|c| c.is_ascii_digit())).collect();
        names.sort();
        for name in names {
            if let Ok(list) = std::fs::read_to_string(format!("/sys/devices/system/node/{}/cpulist", name)) {
                let cpus = parse_cpu_list(&list);
                if !cpus.is_empty() {
                    nodes.push(cpus);
                }
            }
        }
        nodes
    }

    fn core_of(cpu: usize) -> Option<(usize, usize)> {
        let read = |file: &str| std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/topology/{}", cpu, file)).ok()?.trim().parse::<usize>().ok();
        Some((read("physical_package_id")?, read("core_id")?))
    }

    /// Confines the process, and every thread it makes from here on, to the
    /// given CPUs. A failure is left silent: the program still runs, only on
    /// whatever the scheduler picks.
    pub fn pin_to(cpus: &BTreeSet<usize>) {
        // Safe: a zeroed cpu_set_t is the empty set, CPU_SET writes inside
        // it, and sched_setaffinity reads it for the length given.
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            for cpu in cpus {
                if *cpu < libc::CPU_SETSIZE as usize {
                    libc::CPU_SET(*cpu, &mut set);
                }
            }
            let _ = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(list: &str) -> BTreeSet<usize> {
        parse_cpu_list(list)
    }

    /// A four-socket machine as Linux describes it: four nodes of 48 logical
    /// CPUs, where CPU n and CPU n+96 are the two threads of one core.
    fn four_sockets() -> (Vec<BTreeSet<usize>>, impl Fn(usize) -> Option<(usize, usize)>) {
        let nodes = vec![set("0-23,96-119"), set("24-47,120-143"), set("48-71,144-167"), set("72-95,168-191")];
        let core_of = |cpu: usize| Some(((cpu % 96) / 24, cpu % 96));
        (nodes, core_of)
    }

    #[test]
    fn the_kernel_cpu_list_syntax_is_read() {
        assert_eq!(parse_cpu_list("0-3,8,10-11\n"), [0, 1, 2, 3, 8, 10, 11].into_iter().collect());
        assert_eq!(parse_cpu_list("5"), [5].into_iter().collect());
        assert!(parse_cpu_list("").is_empty());
    }

    #[test]
    fn a_four_socket_machine_gets_one_socket_of_physical_cores_and_is_pinned_to_it() {
        let (nodes, core_of) = four_sockets();
        let everything = set("0-191");
        let plan = choose(30, &nodes, &everything, core_of);
        assert_eq!(plan.threads, 24, "one socket's physical cores, not 192 threads");
        assert_eq!(plan.pin_to, Some(set("24-47,120-143")), "pinned to the node CPU 30 lives on");
    }

    #[test]
    fn a_process_already_confined_by_taskset_is_left_where_it_is() {
        let (nodes, core_of) = four_sockets();
        let plan = choose(0, &nodes, &set("0-1"), core_of);
        assert_eq!(plan.threads, 2);
        assert_eq!(plan.pin_to, None, "taskset already decided, nothing to pin");
    }

    #[test]
    fn a_desktop_with_one_node_uses_its_physical_cores_and_is_not_pinned() {
        let nodes = vec![set("0-23")];
        let core_of = |cpu: usize| Some((0, cpu % 12));
        let plan = choose(7, &nodes, &set("0-23"), core_of);
        assert_eq!(plan, Plan { threads: 12, pin_to: None });
    }

    #[test]
    fn a_machine_that_describes_nothing_counts_what_it_is_allowed() {
        let plan = choose(0, &[], &set("0-1"), |_| None);
        assert_eq!(plan, Plan { threads: 2, pin_to: None });
        let plan = choose(0, &[], &BTreeSet::new(), |_| None);
        assert_eq!(plan.threads, 1, "never zero threads");
    }

    /// On whatever machine runs the tests, the plan is something the machine
    /// can honor, and the pool exists afterwards. Run alone with --nocapture
    /// to see what this machine was given.
    #[test]
    fn configure_picks_a_count_the_machine_can_honor() {
        let threads = configure();
        let logical = std::thread::available_parallelism().map(|count| count.get()).unwrap_or(1);
        assert!(threads >= 1 && threads <= logical, "{} threads on a machine with {} logical CPUs", threads, logical);
        assert!(rayon::current_num_threads() >= 1);
        println!("this machine: {} threads for the program, {} in rayon's pool, {} logical CPUs", threads, rayon::current_num_threads(), logical);
    }

    #[test]
    fn a_current_cpu_outside_every_node_falls_back_to_what_is_allowed() {
        let (nodes, core_of) = four_sockets();
        let plan = choose(500, &nodes, &set("0-191"), core_of);
        assert_eq!(plan.threads, 96);
        assert_eq!(plan.pin_to, None);
    }
}
