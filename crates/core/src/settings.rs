//! Desired network configuration, stored separately from observed readiness.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsError {
    InvalidId,
    ReadFailed,
    SaveFailed,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidId => {
                "Enter a nonzero network ID containing exactly 16 hexadecimal characters."
            }
            Self::ReadFailed => "Saved network settings could not be read.",
            Self::SaveFailed => "Network settings could not be saved.",
        })
    }
}

impl std::error::Error for SettingsError {}

impl FromStr for NetworkId {
    type Err = SettingsError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.len() != 16 || !input.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(SettingsError::InvalidId);
        }
        match u64::from_str_radix(input, 16) {
            Ok(value) if value != 0 => Ok(Self(value)),
            _ => Err(SettingsError::InvalidId),
        }
    }
}

impl NetworkId {
    pub fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for NetworkId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:016x}", self.0)
    }
}

/// The caller supplies Android's application-private no-backup directory.
/// This file contains only the desired network ID, never an identity key.
pub fn load(root: &Path) -> Result<Option<NetworkId>, SettingsError> {
    match fs::read_to_string(root.join("network-id")) {
        Ok(value) => value.parse().map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(SettingsError::ReadFailed),
    }
}

pub fn save(root: &Path, input: &str) -> Result<NetworkId, SettingsError> {
    let id: NetworkId = input.parse()?;
    write_private(root, "network-id", format!("{id}\n").as_bytes())
        .map_err(|_| SettingsError::SaveFailed)?;
    Ok(id)
}

pub(crate) fn write_private(root: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    // App JNI calls serialize writes; readers only see the renamed final file.
    let pending = root.join(format!("{name}.pending"));
    let result = (|| -> std::io::Result<()> {
        fs::create_dir_all(root)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&pending)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&pending, root.join(name))?;
        File::open(root)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&pending);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{NetworkId, SettingsError, load, save};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn network_ids_are_exact_and_preserve_all_64_bits() {
        for value in [
            "1",
            "0000000000000000",
            "0123456789abcdeg",
            "0x123456789abcde",
            "１２３４５６７８９０１２３４５６",
        ] {
            assert_eq!(value.parse::<NetworkId>(), Err(SettingsError::InvalidId));
        }
        let id: NetworkId = "  FEDCBA9876543210\n".parse().unwrap();
        assert_eq!(id.to_string(), "fedcba9876543210");
    }

    #[test]
    fn settings_survive_restart_and_invalid_input_does_not_replace_them() {
        let root =
            std::env::temp_dir().join(format!("zeroute-settings-test-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let result = std::panic::catch_unwind(|| {
            assert_eq!(load(&root), Ok(None));
            let id = save(&root, "0123456789ABCDEF").unwrap();
            assert_eq!(load(&root), Ok(Some(id)));
            assert_eq!(save(&root, "bad"), Err(SettingsError::InvalidId));
            assert_eq!(load(&root), Ok(Some(id)));
            assert_eq!(
                fs::metadata(root.join("network-id"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert!(!root.join("network-id.pending").exists());
            fs::write(root.join("network-id"), "corrupt").unwrap();
            assert_eq!(load(&root), Err(SettingsError::InvalidId));
        });
        fs::remove_dir_all(&root).unwrap();
        result.unwrap();
    }
}
