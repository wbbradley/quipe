use std::ffi::CStr;

use libc::strerror;

#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub mod errors {
    use libc::__errno_location;
    pub fn get_errno() -> c_int {
        unsafe { *__errno_location() }
    }
}

#[allow(dead_code)]
#[cfg(target_os = "macos")]
pub mod errors {
    use libc::__error;
    pub fn get_errno() -> libc::c_int {
        unsafe { *__error() }
    }
}

#[derive(Copy, Clone)]
pub struct Errno {
    errno: libc::c_int,
}

impl Errno {
    pub fn latest() -> Self {
        Self {
            errno: errors::get_errno(),
        }
    }
    pub fn is_enoent(self) -> bool {
        self.errno == libc::ENOENT
    }
    pub fn is_eagain(self) -> bool {
        self.errno == libc::EAGAIN
    }
    pub fn is_error(self) -> bool {
        self.errno != 0
    }
}

impl std::fmt::Display for Errno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = (*self).into();
        write!(f, "{}", s)
    }
}

impl From<i32> for Errno {
    fn from(errno: libc::c_int) -> Self {
        Self { errno }
    }
}

impl From<Errno> for String {
    fn from(errno: Errno) -> Self {
        String::from_utf8_lossy(unsafe { CStr::from_ptr(strerror(errno.errno)) }.to_bytes())
            .to_string()
    }
}
