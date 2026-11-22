//! One load/save for the small JSON preference files: size cap, parse,
//! validity check, then a temp-file write so a failed save leaves the old
//! file alone. Each preference type says only what "valid" means.
use serde::{Serialize, de::DeserializeOwned};
use std::{fs, io, path::Path};

pub(crate) trait PrefsFile: Serialize + DeserializeOwned + Default {
    /// Files larger than this are ignored on load.
    const MAX_BYTES: u64 = 4096;
    /// The error `save` returns for an invalid value.
    const INVALID: &'static str;
    fn valid(&self) -> bool;
    /// A hook to bring an older valid file up to date after loading.
    fn upgrade(self) -> Self {
        self
    }
    /// The preferences in `path`, or None if the file is missing or invalid.
    fn read(path: &Path) -> Option<Self> {
        fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= Self::MAX_BYTES)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(Self::valid)
            .map(Self::upgrade)
    }
    /// The preferences in `path`, or the defaults.
    fn load(path: &Path) -> Self {
        Self::read(path).unwrap_or_default()
    }
    fn save(&self, path: &Path) -> io::Result<()> {
        if !self.valid() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, Self::INVALID));
        }
        write_json(self, path)
    }
}

/// Write `value` as JSON to `path` through a temp file next to it.
pub(crate) fn write_json(value: &impl Serialize, path: &Path) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
    crate::resource_write::write_bytes(path, &temp, &bytes)
}
