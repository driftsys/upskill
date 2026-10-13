//! Qualified filesystem profiles and descriptor mount bindings.
use anyhow::{Context, Result, bail};
#[cfg(target_os = "linux")]
use rustix::fs::AtFlags;
use rustix::fs::fstatfs;
use std::fs::File;
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Mount {
    #[cfg(target_os = "linux")]
    id: u64,
    #[cfg(target_os = "macos")]
    fsid: rustix::fs::Fsid,
    #[cfg(target_os = "macos")]
    on: Vec<u8>,
    #[cfg(target_os = "macos")]
    from: Vec<u8>,
}
#[cfg(any(target_os = "linux", test))]
fn linux_profile(magic: u64) -> Result<()> {
    if magic != 0xef53 && magic != 0x01021994 {
        bail!("unqualified Linux filesystem profile {magic:#x}");
    }
    Ok(())
}
#[cfg(any(target_os = "linux", test))]
fn linux_mount_id(mask: u32, id: u64) -> Result<u64> {
    if mask & MOUNT_ID_MASK == 0 {
        bail!("Linux statx mount-ID evidence is unavailable");
    }
    Ok(id)
}
#[cfg(target_os = "macos")]
fn apfs_profile(name: &[u8]) -> Result<()> {
    if name != b"apfs" {
        bail!("unqualified macOS filesystem profile");
    }
    Ok(())
}
#[cfg(target_os = "linux")]
const MOUNT_ID_MASK: u32 = rustix::fs::StatxFlags::MNT_ID.bits();
// Linux STATX_MNT_ID ABI bit, used only for metadata-validator tests on macOS.
#[cfg(all(test, not(target_os = "linux")))]
const MOUNT_ID_MASK: u32 = 0x1000;
fn require_same_mount<T: PartialEq>(expected: &T, actual: &T) -> Result<()> {
    if expected != actual {
        bail!("destination crosses or changes the qualified mount binding");
    }
    Ok(())
}
#[cfg(target_os = "macos")]
pub(super) type DescriptorQueries = fn(&File) -> rustix::io::Result<rustix::fs::StatFs>;
#[cfg(target_os = "linux")]
pub(super) type DescriptorQueries = (
    fn(&File) -> rustix::io::Result<rustix::fs::StatFs>,
    fn(&File) -> rustix::io::Result<rustix::fs::Statx>,
);
impl Mount {
    pub(super) fn queries() -> DescriptorQueries {
        #[cfg(target_os = "macos")]
        {
            |file| fstatfs(file)
        }
        #[cfg(target_os = "linux")]
        {
            (
                |file| fstatfs(file),
                |file| {
                    rustix::fs::statx(
                        file,
                        "",
                        AtFlags::EMPTY_PATH,
                        rustix::fs::StatxFlags::MNT_ID,
                    )
                },
            )
        }
    }
    pub(super) fn of(file: &File, queries: DescriptorQueries) -> Result<Self> {
        #[cfg(target_os = "macos")]
        let profile_query = queries;
        #[cfg(target_os = "linux")]
        let (profile_query, mount_query) = queries;
        let metadata = profile_query(file).context("qualify descriptor filesystem profile")?;
        #[cfg(target_os = "linux")]
        {
            linux_profile(metadata.f_type as u64)?;
            let stat = mount_query(file).context("obtain Linux statx mount-ID evidence")?;
            Ok(Self {
                id: linux_mount_id(stat.stx_mask, stat.stx_mnt_id)?,
            })
        }
        #[cfg(target_os = "macos")]
        {
            fn native_bytes(chars: &[std::ffi::c_char]) -> Vec<u8> {
                chars
                    .iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8)
                    .collect()
            }
            apfs_profile(&native_bytes(&metadata.f_fstypename))?;
            Ok(Self {
                fsid: metadata.f_fsid,
                on: native_bytes(&metadata.f_mntonname),
                from: native_bytes(&metadata.f_mntfromname),
            })
        }
    }
    pub(super) fn require_same(&self, other: &Self) -> Result<()> {
        require_same_mount(self, other)
    }
}
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    #[test]
    fn unknown_linux_profile_is_rejected() {
        assert!(
            linux_profile(0x794c7630).is_err(),
            "overlay profile accepted"
        );
        assert!(linux_profile(0).is_err());
        linux_profile(0xef53).unwrap();
        linux_profile(0x01021994).unwrap();
    }
    #[test]
    fn missing_linux_mount_id_mask_is_rejected() {
        assert!(
            linux_mount_id(0, 77).is_err(),
            "missing returned mount-ID mask accepted"
        );
        assert_eq!(linux_mount_id(MOUNT_ID_MASK, 77).unwrap(), 77);
    }
    #[test]
    fn different_mount_id_on_same_device_is_rejected() {
        let expected = (123_u64, 77_u64);
        let other_mount = (123_u64, 78_u64);
        assert!(
            require_same_mount(&expected, &other_mount).is_err(),
            "same device hid different mount ID"
        );
        require_same_mount(&expected, &expected).unwrap();
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn unknown_macos_profile_is_rejected() {
        assert!(
            apfs_profile(b"hfs").is_err(),
            "unknown macOS profile accepted"
        );
        apfs_profile(b"apfs").unwrap();
    }
    pub(in crate::filesystem::destinations) fn different_binding(mount: &Mount) -> Mount {
        #[cfg(target_os = "macos")]
        {
            let mut on = mount.on.clone();
            on.push(b'x');
            Mount {
                fsid: mount.fsid,
                on,
                from: mount.from.clone(),
            }
        }
        #[cfg(target_os = "linux")]
        {
            Mount { id: mount.id ^ 1 }
        }
    }
    #[test]
    fn production_entry_rejects_unknown_native_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let file = File::open(tmp.path()).unwrap();
        Mount::of(&file, Mount::queries()).unwrap();
        let unknown = |file: &File| {
            let mut metadata = fstatfs(file)?;
            #[cfg(target_os = "macos")]
            {
                metadata.f_fstypename.fill(0);
                metadata.f_fstypename[0] = b'x' as std::ffi::c_char;
            }
            #[cfg(target_os = "linux")]
            {
                metadata.f_type = 0x794c7630;
            }
            Ok(metadata)
        };
        #[cfg(target_os = "macos")]
        let queries: DescriptorQueries = unknown;
        #[cfg(target_os = "linux")]
        let queries: DescriptorQueries = (unknown, Mount::queries().1);
        assert!(
            Mount::of(&file, queries).is_err(),
            "production qualification accepted unknown native profile"
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn production_entry_rejects_missing_returned_mount_id_mask() {
        let tmp = tempfile::tempdir().unwrap();
        let file = File::open(tmp.path()).unwrap();
        Mount::of(&file, Mount::queries()).unwrap();
        let missing = |file: &File| {
            let mut stat = Mount::queries().1(file)?;
            stat.stx_mask &= !MOUNT_ID_MASK;
            stat.stx_mnt_id = 77;
            assert_eq!(stat.stx_mask & MOUNT_ID_MASK, 0);
            assert_eq!(stat.stx_mnt_id, 77);
            Ok(stat)
        };
        assert!(
            Mount::of(&file, (Mount::queries().0, missing)).is_err(),
            "production qualification accepted missing returned mount-ID mask"
        );
    }
}
