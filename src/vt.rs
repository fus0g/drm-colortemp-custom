//! Active VT (virtual terminal) detection via VT_GETSTATE ioctl.

use nix::ioctl_read_bad;
use std::ffi::CString;

#[repr(C)]
#[derive(Default)]
pub struct VtStat {
    pub v_active: u16,
    pub v_signal: u16,
    pub v_state: u16,
}

// VT_GETSTATE = 0x5603 (from <linux/vt.h>). The kernel uses raw numbers, not _IO macros.
ioctl_read_bad!(vt_getstate, 0x5603, VtStat);

/// Returns the active VT number (1-based) or None if the kernel won't tell us
/// (no console fd, ioctl unsupported, e.g. on systems without classic VTs).
pub fn active_vt() -> Option<i32> {
    let fd = open_console()?;
    let mut st = VtStat::default();
    let res = unsafe { vt_getstate(fd, &mut st) };
    unsafe { libc::close(fd) };
    match res {
        Ok(_) => Some(st.v_active as i32),
        Err(_) => None,
    }
}

fn open_console() -> Option<i32> {
    for path in &["/dev/tty0", "/dev/console"] {
        let c = CString::new(*path).ok()?;
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR) };
        if fd >= 0 {
            return Some(fd);
        }
        let fd_ro = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) };
        if fd_ro >= 0 {
            return Some(fd_ro);
        }
    }
    None
}

/// Switch the active Linux virtual terminal (VT) via VT_ACTIVATE / VT_WAITACTIVE.
/// Requires root / console access.
pub fn switch_vt(vt_num: i32) -> Result<(), std::io::Error> {
    for path in &["/dev/tty0", "/dev/console"] {
        if let Ok(c) = CString::new(*path) {
            let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR) };
            if fd >= 0 {
                // VT_ACTIVATE = 0x5606, VT_WAITACTIVE = 0x5607
                let r1 = unsafe { libc::ioctl(fd, 0x5606, vt_num as libc::c_long) };
                let r2 = unsafe { libc::ioctl(fd, 0x5607, vt_num as libc::c_long) };
                unsafe { libc::close(fd) };
                if r1 == 0 && r2 == 0 {
                    return Ok(());
                }
            }
        }
    }
    Err(std::io::Error::last_os_error())
}

