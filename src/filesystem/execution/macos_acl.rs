//! Minimal Darwin ACL inspection; no ACL mutation or additional dependency.
use anyhow::{Result, ensure};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::os::fd::AsRawFd;

// Darwin sys/acl.h declares opaque acl_t/acl_entry_t pointers and these C ABIs.
unsafe extern "C" {
    fn acl_get_fd_np(fd: c_int, acl_type: c_int) -> *mut c_void;
    fn acl_get_entry(acl: *mut c_void, entry_id: c_int, entry: *mut *mut c_void) -> c_int;
    fn acl_free(object: *mut c_void) -> c_int;
}
struct Acl(*mut c_void);
impl Drop for Acl {
    fn drop(&mut self) {
        // SAFETY: this non-null allocation came from acl_get_fd_np and is freed once.
        unsafe {
            acl_free(self.0);
        }
    }
}
pub(super) fn require_empty(file: &File) -> Result<()> {
    const ACL_TYPE_EXTENDED: c_int = 0x100;
    const ACL_FIRST_ENTRY: c_int = 0;
    // SAFETY: file retains a live descriptor; the type is Darwin's declared enum value.
    let raw = unsafe { acl_get_fd_np(file.as_raw_fd(), ACL_TYPE_EXTENDED) };
    if raw.is_null() {
        let error = std::io::Error::last_os_error();
        // Darwin Libc filesec_get_property reports ENOENT for an absent FILESEC_ACL.
        // acl_get_fd_np obtains this property through fstatx_np on the live descriptor.
        if error.raw_os_error() == Some(rustix::io::Errno::NOENT.raw_os_error()) {
            file.metadata()?;
            return Ok(());
        }
        anyhow::bail!("cannot qualify native ACL: {error}");
    }
    let acl = Acl(raw);
    let mut entry = std::ptr::null_mut();
    // SAFETY: acl is a live API allocation; entry is an initialized writable pointer slot.
    let result = unsafe { acl_get_entry(acl.0, ACL_FIRST_ENTRY, &mut entry) };
    let error = std::io::Error::last_os_error();
    ensure!(result != 0, "ACL metadata is unsupported for execution");
    // For a valid freshly retrieved ACL, FIRST_ENTRY fails with EINVAL when empty.
    ensure!(
        result == -1 && error.raw_os_error() == Some(rustix::io::Errno::INVAL.raw_os_error()),
        "cannot qualify empty native ACL: {error}"
    );
    Ok(())
}
