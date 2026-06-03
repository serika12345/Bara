use std::{num::NonZeroUsize, ptr::NonNull};

#[derive(Debug)]
pub struct ExecutableMemory {
    ptr: NonNull<u8>,
    len: NonZeroUsize,
}

impl ExecutableMemory {
    #[cfg(all(unix, target_arch = "aarch64"))]
    pub fn allocate(code: &[u8]) -> Result<Self, ExecutableMemoryError> {
        let len = NonZeroUsize::new(code.len()).ok_or(ExecutableMemoryError::EmptyCode)?;

        // Safety: mmap is called with a null address hint, private anonymous mapping,
        // and a non-zero length. The returned pointer is checked against MAP_FAILED.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len.get(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(ExecutableMemoryError::AllocateFailed);
        }

        let ptr = NonNull::new(ptr.cast::<u8>()).ok_or(ExecutableMemoryError::AllocateFailed)?;

        // Safety: ptr is a valid writable mapping of at least len bytes, and code
        // points to len initialized bytes that do not overlap the destination.
        unsafe {
            std::ptr::copy_nonoverlapping(code.as_ptr(), ptr.as_ptr(), len.get());
        }

        // Safety: ptr and len describe the live mapping created above.
        let protect_result = unsafe {
            libc::mprotect(
                ptr.as_ptr().cast(),
                len.get(),
                libc::PROT_READ | libc::PROT_EXEC,
            )
        };

        if protect_result != 0 {
            // Safety: ptr and len still describe the mapping from mmap.
            let _ = unsafe { libc::munmap(ptr.as_ptr().cast(), len.get()) };
            return Err(ExecutableMemoryError::ProtectFailed);
        }

        Ok(Self { ptr, len })
    }

    #[cfg(not(all(unix, target_arch = "aarch64")))]
    pub fn allocate(code: &[u8]) -> Result<Self, ExecutableMemoryError> {
        let _ = code;
        Err(ExecutableMemoryError::UnsupportedHost)
    }

    pub const fn entry_ptr(&self) -> NonNull<u8> {
        self.ptr
    }

    pub const fn len(&self) -> NonZeroUsize {
        self.len
    }
}

#[cfg(all(unix, target_arch = "aarch64"))]
impl Drop for ExecutableMemory {
    fn drop(&mut self) {
        // Safety: self.ptr/self.len were returned by mmap and are owned by this value.
        let _ = unsafe { libc::munmap(self.ptr.as_ptr().cast(), self.len.get()) };
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableMemoryError {
    EmptyCode,
    AllocateFailed,
    ProtectFailed,
    UnsupportedHost,
}
