//! What processes cost the machine: CPU and resident memory of one process, or of some
//! processes and every process they started (add-usage-window D1).

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// CPU and memory of one process or a tree of them.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Footprint {
    /// Percent of one core, as Activity Monitor shows it, so a busy tree can pass 100.
    /// `None` when the sample before it is missing or too old to compare with.
    pub cpu: Option<f32>,
    /// Resident memory in bytes, summed over the processes.
    pub memory: u64,
    /// How many processes it covers, the first one included.
    pub processes: u32,
}

impl Footprint {
    /// The two together, as one figure.
    pub fn plus(self, other: Footprint) -> Footprint {
        let cpu = match (self.cpu, other.cpu) {
            (Some(one), Some(two)) => Some(one + two),
            _ => None,
        };
        Footprint {
            cpu,
            memory: self.memory + other.memory,
            processes: self.processes + other.processes,
        }
    }
}

/// CPU use is averaged since the previous sample, so one older than this would blur a
/// long stretch into one figure; the Usage window samples every 2 s.
const FRESH: Duration = Duration::from_secs(10);

/// Every process `root` started, directly or not, now. Used when a background command
/// starts, to tell its processes apart from the ones already there
/// (add-background-work-status D6).
pub fn descendants(root: u32) -> HashSet<u32> {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
    for process in system.processes().values() {
        if process.thread_kind().is_some() {
            continue;
        }
        if let Some(parent) = process.parent() {
            children.entry(parent).or_default().push(process.pid());
        }
    }
    let mut found = HashSet::new();
    let mut pending = vec![Pid::from_u32(root)];
    while let Some(pid) = pending.pop() {
        for child in children.get(&pid).into_iter().flatten() {
            if found.insert(child.as_u32()) {
                pending.push(*child);
            }
        }
    }
    found
}

/// Which of `pids` still run.
pub fn running(pids: &[u32]) -> HashSet<u32> {
    let wanted: Vec<Pid> = pids.iter().copied().map(Pid::from_u32).collect();
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&wanted),
        true,
        ProcessRefreshKind::nothing(),
    );
    pids.iter()
        .copied()
        .filter(|pid| system.process(Pid::from_u32(*pid)).is_some())
        .collect()
}

/// Keeps the process table between samples, because CPU use is measured between two.
pub struct Sampler {
    system: System,
    last: Option<Instant>,
    /// Whether the latest sample followed a fresh one, so its CPU figures mean something.
    cpu_ready: bool,
    children: HashMap<Pid, Vec<Pid>>,
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler::new()
    }
}

impl Sampler {
    pub fn new() -> Sampler {
        Sampler {
            system: System::new(),
            last: None,
            cpu_ready: false,
            children: HashMap::new(),
        }
    }

    /// Reads every process's CPU, memory and parent now.
    pub fn refresh(&mut self) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
        self.cpu_ready = self.last.is_some_and(|at| at.elapsed() <= FRESH);
        self.last = Some(Instant::now());
        self.children.clear();
        for process in self.system.processes().values() {
            // Linux lists threads as tasks of their process; their memory is the
            // process's own.
            if process.thread_kind().is_some() {
                continue;
            }
            if let Some(parent) = process.parent() {
                self.children.entry(parent).or_default().push(process.pid());
            }
        }
    }

    /// The process alone, or `None` when it is not running.
    pub fn alone(&self, pid: u32) -> Option<Footprint> {
        let process = self.system.process(Pid::from_u32(pid))?;
        Some(Footprint {
            cpu: self.cpu(process),
            memory: process.memory(),
            processes: 1,
        })
    }

    /// The processes and everything they started, each counted once, or `None` when
    /// none of them is running.
    pub fn tree(&self, roots: &[u32]) -> Option<Footprint> {
        let mut pending: Vec<Pid> = roots
            .iter()
            .map(|pid| Pid::from_u32(*pid))
            .filter(|pid| self.system.process(*pid).is_some())
            .collect();
        if pending.is_empty() {
            return None;
        }
        let mut seen = std::collections::HashSet::new();
        let mut total = Footprint {
            cpu: self.cpu_ready.then_some(0.0),
            memory: 0,
            processes: 0,
        };
        while let Some(pid) = pending.pop() {
            if !seen.insert(pid) {
                continue;
            }
            let Some(process) = self.system.process(pid) else {
                continue;
            };
            total = total.plus(Footprint {
                cpu: self.cpu(process),
                memory: process.memory(),
                processes: 1,
            });
            if let Some(children) = self.children.get(&pid) {
                pending.extend(children);
            }
        }
        Some(total)
    }

    fn cpu(&self, process: &sysinfo::Process) -> Option<f32> {
        self.cpu_ready.then(|| process.cpu_usage())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    #[test]
    fn a_tree_counts_what_its_root_started() {
        let mut parent = Command::new("/bin/sh")
            .args(["-c", "sleep 30 & sleep 30 & wait"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let pid = parent.id();
        let mut sampler = Sampler::new();
        let mut tree = None;
        for _ in 0..50 {
            sampler.refresh();
            tree = sampler.tree(&[pid]);
            if tree.is_some_and(|tree| tree.processes >= 3) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let tree = tree.unwrap();
        let alone = sampler.alone(pid).unwrap();
        let _ = parent.kill();
        let _ = parent.wait();

        assert_eq!(tree.processes, 3, "the shell and its two sleeps");
        assert_eq!(alone.processes, 1);
        assert!(tree.memory > alone.memory);
        assert!(tree.cpu.is_some(), "later samples have CPU");
        assert_eq!(
            sampler.tree(&[pid, pid]).unwrap().processes,
            3,
            "a root named twice counts once"
        );
    }

    #[test]
    fn the_first_sample_has_no_cpu_and_gone_processes_have_nothing() {
        let mut sampler = Sampler::new();
        sampler.refresh();
        let me = sampler.alone(std::process::id()).unwrap();
        assert_eq!(me.cpu, None);
        assert!(me.memory > 0);
        assert_eq!(sampler.tree(&[u32::MAX - 1]), None);
        assert_eq!(sampler.alone(u32::MAX - 1), None);
    }

    #[test]
    fn a_process_and_what_it_started_are_found() {
        let mut parent = std::process::Command::new("sh")
            .args(["-c", "sleep 5 & wait"])
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let found = descendants(parent.id());
        assert!(!found.is_empty(), "the sleep under the shell");
        let all: Vec<u32> = found.iter().copied().collect();
        assert_eq!(running(&all), found);
        let _ = parent.kill();
        let _ = parent.wait();
        assert!(running(&[parent.id()]).is_empty(), "gone once reaped");
    }

    #[test]
    fn figures_add_up() {
        let one = Footprint {
            cpu: Some(10.0),
            memory: 100,
            processes: 1,
        };
        let two = Footprint {
            cpu: Some(5.5),
            memory: 50,
            processes: 2,
        };
        assert_eq!(
            one.plus(two),
            Footprint {
                cpu: Some(15.5),
                memory: 150,
                processes: 3
            }
        );
        assert_eq!(
            one.plus(Footprint::default()).cpu,
            None,
            "unknown stays unknown"
        );
    }
}
