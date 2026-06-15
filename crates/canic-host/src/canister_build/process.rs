use std::fs;

pub(super) fn parent_process_id() -> Option<u32> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    parse_parent_process_id(&stat)
}

// Walk ancestor processes until the wrapping `icp` process is found.
pub(super) fn icp_ancestor_process_id() -> Option<u32> {
    let mut pid = parent_process_id()?;
    loop {
        if process_comm(pid).as_deref() == Some("icp") {
            return Some(pid);
        }

        let parent = process_parent_id(pid)?;
        if parent == 0 || parent == pid {
            return None;
        }
        pid = parent;
    }
}

// Read one ancestor's parent process id from procfs.
fn process_parent_id(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_parent_process_id(&stat)
}

// Read one process command name from procfs.
fn process_comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|comm| comm.trim().to_string())
}

// Parse Linux `/proc/<pid>/stat` enough to extract the parent process id.
pub(super) fn parse_parent_process_id(stat: &str) -> Option<u32> {
    let (_, suffix) = stat.rsplit_once(") ")?;
    let mut parts = suffix.split_whitespace();
    let _state = parts.next()?;
    parts.next()?.parse::<u32>().ok()
}
