use crate::{ExecutableMemory, ExecutableMemoryError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResult {
    return_value: u64,
}

impl RunResult {
    pub const fn new(return_value: u64) -> Self {
        Self { return_value }
    }

    pub const fn return_value(&self) -> u64 {
        self.return_value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunError {
    ExecutableMemory(ExecutableMemoryError),
    UnsupportedHost,
}

pub fn run_no_args_u64(code: &[u8]) -> Result<RunResult, RunError> {
    let executable = ExecutableMemory::allocate(code).map_err(RunError::ExecutableMemory)?;
    call_no_args_u64(&executable)
}

#[cfg(all(unix, target_arch = "aarch64"))]
fn call_no_args_u64(executable: &ExecutableMemory) -> Result<RunResult, RunError> {
    type GeneratedFunction = extern "C" fn() -> u64;

    // Safety: ExecutableMemory only exposes a pointer to code copied into an
    // executable mapping. This runner exposes only the no-args u64-return ABI.
    let function: GeneratedFunction = unsafe { std::mem::transmute(executable.entry_ptr()) };
    Ok(RunResult::new(function()))
}

#[cfg(not(all(unix, target_arch = "aarch64")))]
fn call_no_args_u64(executable: &ExecutableMemory) -> Result<RunResult, RunError> {
    let _ = executable;
    Err(RunError::UnsupportedHost)
}
