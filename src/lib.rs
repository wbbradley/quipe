use std::{
    ffi::CString,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, io::RawFd},
    },
    path::Path,
};

use self::{errno::Errno, error::*};

mod errno;
mod error;

pub struct PipeQueue {
    write_fd: RawFd,
}

impl AsRawFd for PipeQueue {
    fn as_raw_fd(&self) -> RawFd {
        self.write_fd
    }
}

pub struct PipeReader {
    read_fd: RawFd,
}

fn open(path: &Path, flags: libc::c_int, mode: libc::c_int) -> Result<RawFd> {
    let fd = unsafe {
        libc::open(
            path.as_os_str().as_bytes().as_ptr() as *const i8,
            flags,
            mode,
        )
    };
    if fd < 0 {
        Err(Error::new(format!(
            "Failed to open file at {} [errno={errno}]",
            path.display(),
            errno = Errno::from(fd)
        )))
    } else {
        Ok(fd)
    }
}

fn mkfifo(path: &Path, mode: libc::mode_t) -> Result<()> {
    let result = unsafe {
        libc::mkfifo(
            CString::new(path.as_os_str().as_bytes()).unwrap().as_ptr(),
            mode,
        )
    };
    if result < 0 {
        Err(Error::new(format!(
            "failed to create FIFO at {} [errno={errno}]",
            path.display(),
            errno = Errno::latest(),
        )))
    } else {
        Ok(())
    }
}

fn read_all(fd: RawFd, mut data: &mut [u8]) -> Result<()> {
    while !data.is_empty() {
        match unsafe { libc::read(fd, data.as_mut_ptr() as *mut libc::c_void, data.len()) } {
            0 => {
                return Err(Error::new("failed to read all bytes"));
            }
            -1 => {
                if Errno::latest().is_eagain() {
                    continue;
                } else {
                    return Err(Error::new(format!(
                        "failed to read [errno={errno}]",
                        errno = Errno::latest(),
                    )));
                }
            }
            n => {
                assert!(n > 0, "undefined behavior from POSIX read!");
                let n = n as usize;
                data = &mut data[n..];
            }
        }
    }
    assert!(data.is_empty());
    Ok(())
}

fn write_all(fd: RawFd, mut data: &[u8]) -> Result<()> {
    while !data.is_empty() {
        match unsafe { libc::write(fd, data.as_ptr() as *const libc::c_void, data.len()) } {
            0 => {
                return Err(Error::new("failed to write all bytes [errno={errno}]"));
            }
            -1 => {
                if Errno::latest().is_eagain() {
                    continue;
                } else {
                    return Err(Error::new(format!(
                        "failed to write [errno={errno}]",
                        errno = Errno::latest(),
                    )));
                }
            }
            n => {
                assert!(n > 0, "undefined behavior from POSIX write!");
                let n = n as usize;
                data = &data[n..];
            }
        }
    }
    assert!(data.is_empty());
    Ok(())
}

struct AdvisoryLock {
    fd: RawFd,
}

impl AdvisoryLock {
    fn new(fd: RawFd) -> Self {
        Self { fd }
    }
}

impl Drop for AdvisoryLock {
    fn drop(&mut self) {
        flock(self.fd, libc::LOCK_UN).expect("failed to release lock on pipe");
    }
}

fn flock(fd: RawFd, operation: libc::c_int) -> Result<()> {
    let result = unsafe { libc::flock(fd, operation) };
    if result < 0 {
        Err(Error::new(format!(
            "failed to acquire lock on pipe [errno={errno}]",
            errno = Errno::latest(),
        )))
    } else {
        Ok(())
    }
}
impl PipeQueue {
    pub fn create(path: &Path) -> Result<Self> {
        mkfifo(path, libc::S_IRWXU)?;
        let write_fd = open(path, libc::O_WRONLY | libc::O_NONBLOCK, 0)?;
        Ok(PipeQueue { write_fd })
    }

    pub fn send(&self, data: &[u8]) -> Result<()> {
        // First byte is the message length
        let mut message = Vec::with_capacity(std::mem::size_of::<u32>() + data.len());
        message.extend_from_slice(
            &u32::try_from(data.len())
                .expect("message too long")
                .to_be_bytes(),
        );
        message.extend_from_slice(data);
        write_all(self.write_fd, message.as_slice())
    }
}

impl PipeReader {
    pub fn new(path: &Path) -> Result<Self> {
        let read_fd = open(path, libc::O_RDONLY | libc::O_NONBLOCK, 0)?;
        Ok(PipeReader { read_fd })
    }

    pub fn receive(&self) -> Result<Vec<u8>> {
        let _advisory_lock = AdvisoryLock::new(self.read_fd);
        self.read_message()
    }

    fn read_message(&self) -> Result<Vec<u8>> {
        // Read the length.
        let mut len_buf = [0u8; 4];
        read_all(self.read_fd, &mut len_buf)?;
        // Allocate space.
        let msg_len = u32::from_be_bytes(len_buf);
        // Read the content.
        let mut buffer = vec![0u8; msg_len as usize];
        read_all(self.read_fd, buffer.as_mut_slice())?;
        Ok(buffer)
    }
}

impl Drop for PipeQueue {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.write_fd) };
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.read_fd) };
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_mainline_scenario() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("my_queue");
        let queue = PipeQueue::create(&path).unwrap();

        // Create multiple readers
        let reader1 = PipeReader::new(&path).unwrap();
        let reader2 = PipeReader::new(&path).unwrap();

        // Spawn reader threads
        let handle1 = thread::spawn(move || {
            let data = reader1.receive().unwrap();
            assert!(data == b"Hello, reader!");
        });

        let handle2 = thread::spawn(move || {
            let data = reader2.receive().unwrap();
            assert!(data == b"Hello, reader!");
        });

        // Send a message
        queue.send(b"Hello, reader!").unwrap();
        queue.send(b"Hello, reader!").unwrap();

        handle1.join().unwrap();
        handle2.join().unwrap();
    }
}
