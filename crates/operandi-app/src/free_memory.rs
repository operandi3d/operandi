//! Memory free for Operandi now, for checks before big edits (Subdivide).
//! "Available" memory: free plus what the system can reclaim at once
//! (on macOS the inactive and purgeable pages), as `sysinfo` reports it.

/// Free memory in bytes; `u64::MAX` (no limit) when it can't be read.
pub fn free_bytes() -> u64 {
    match sysinfo::System::new() {
        Ok(mut system) => {
            system.refresh_memory_specifics(sysinfo::MemoryRefreshKind::nothing().with_ram());
            match system.available_memory() {
                0 => u64::MAX,
                n => n,
            }
        }
        Err(_) => u64::MAX,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn free_memory_reads_something_plausible() {
        let free = super::free_bytes();
        // More than 16 MB and, when read, less than 64 TB.
        assert!(free > 16 << 20);
        assert!(free == u64::MAX || free < 64 << 40);
    }
}
