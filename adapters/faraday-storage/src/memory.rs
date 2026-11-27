//! The memory read (02 §4.7): what is free for a vault's Argon2id, in MiB.

/// The memory available, in MiB: `/proc/meminfo`'s `MemAvailable`, less than the control group's
/// limit when one is set.
pub fn available_mib() -> Option<u32> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut mib = meminfo_available_mib(&info)?;
    let max = std::fs::read_to_string("/sys/fs/cgroup/memory.max").ok();
    let current = std::fs::read_to_string("/sys/fs/cgroup/memory.current").ok();
    if let Some(free) = max.and_then(|m| cgroup_free_mib(&m, current.as_deref())) {
        mib = mib.min(free);
    }
    Some(mib)
}

fn meminfo_available_mib(info: &str) -> Option<u32> {
    let line = info.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    u32::try_from(kib / 1024).ok()
}

/// The control group's limit less its use; `None` for no limit.
fn cgroup_free_mib(max: &str, current: Option<&str>) -> Option<u32> {
    let max: u64 = max.trim().parse().ok()?;
    let used: u64 = current.and_then(|c| c.trim().parse().ok()).unwrap_or(0);
    u32::try_from(max.saturating_sub(used) / (1024 * 1024)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: a control group's limit leaves its limit less its use, and no limit leaves none.
    /// Regression: Argon2id sized to the machine's memory while the group allows less, and killed for it.
    /// Owner: adapter
    #[test]
    fn a_cgroup_limit_leaves_its_limit_less_its_use_and_no_limit_leaves_none() {
        assert_eq!(
            cgroup_free_mib("1073741824\n", Some("268435456\n")),
            Some(768)
        );
        assert_eq!(cgroup_free_mib("max\n", Some("1\n")), None);
        assert_eq!(meminfo_available_mib("MemAvailable:  2048 kB\n"), Some(2));
    }
}
