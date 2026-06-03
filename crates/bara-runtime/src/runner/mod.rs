use crate::{ExecutableMemory, ExecutableMemoryError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResult {
    return_value: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunArgumentU64(u64);

impl RunArgumentU64 {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputMemory {
    bytes: Vec<u8>,
}

impl InputMemory {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, InputMemoryError> {
        if bytes.is_empty() {
            return Err(InputMemoryError::Empty);
        }

        Ok(Self { bytes })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputMemoryError {
    Empty,
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

pub fn run_one_u64(code: &[u8], argument: RunArgumentU64) -> Result<RunResult, RunError> {
    let executable = ExecutableMemory::allocate(code).map_err(RunError::ExecutableMemory)?;
    call_one_u64(&executable, argument)
}

pub fn run_one_input_memory_ptr(code: &[u8], memory: InputMemory) -> Result<RunResult, RunError> {
    let executable = ExecutableMemory::allocate(code).map_err(RunError::ExecutableMemory)?;
    call_one_input_memory_ptr(&executable, &memory)
}

#[cfg(all(unix, target_arch = "aarch64"))]
fn call_no_args_u64(executable: &ExecutableMemory) -> Result<RunResult, RunError> {
    type GeneratedFunction = extern "C" fn() -> u64;

    // Safety: ExecutableMemory only exposes a pointer to code copied into an
    // executable mapping. This runner exposes only the no-args u64-return ABI.
    let function: GeneratedFunction = unsafe { std::mem::transmute(executable.entry_ptr()) };
    Ok(RunResult::new(function()))
}

#[cfg(all(unix, target_arch = "aarch64"))]
fn call_one_u64(
    executable: &ExecutableMemory,
    argument: RunArgumentU64,
) -> Result<RunResult, RunError> {
    type GeneratedFunction = extern "C" fn(u64) -> u64;

    // Safety: ExecutableMemory only exposes a pointer to code copied into an
    // executable mapping. This runner exposes only the one-u64-arg u64-return ABI.
    let function: GeneratedFunction = unsafe { std::mem::transmute(executable.entry_ptr()) };
    Ok(RunResult::new(function(argument.0)))
}

#[cfg(all(unix, target_arch = "aarch64"))]
fn call_one_input_memory_ptr(
    executable: &ExecutableMemory,
    memory: &InputMemory,
) -> Result<RunResult, RunError> {
    type GeneratedFunction = extern "C" fn(*const u8) -> u64;

    // Safety: ExecutableMemory only exposes a pointer to code copied into an
    // executable mapping. The input memory slice is kept alive for the call and
    // passed read-only to the generated one-pointer-arg u64-return function.
    let function: GeneratedFunction = unsafe { std::mem::transmute(executable.entry_ptr()) };
    Ok(RunResult::new(function(memory.bytes().as_ptr())))
}

#[cfg(not(all(unix, target_arch = "aarch64")))]
fn call_no_args_u64(executable: &ExecutableMemory) -> Result<RunResult, RunError> {
    let _ = executable;
    Err(RunError::UnsupportedHost)
}

#[cfg(not(all(unix, target_arch = "aarch64")))]
fn call_one_u64(
    executable: &ExecutableMemory,
    argument: RunArgumentU64,
) -> Result<RunResult, RunError> {
    let _ = executable;
    let _ = argument;
    Err(RunError::UnsupportedHost)
}

#[cfg(not(all(unix, target_arch = "aarch64")))]
fn call_one_input_memory_ptr(
    executable: &ExecutableMemory,
    memory: &InputMemory,
) -> Result<RunResult, RunError> {
    let _ = executable;
    let _ = memory;
    Err(RunError::UnsupportedHost)
}

#[cfg(test)]
mod tests {
    use crate::{InputMemory, InputMemoryError, RunArgumentU64, RunResult};

    #[test]
    fn run_result_exposes_return_value() {
        assert_eq!(RunResult::new(42).return_value(), 42);
    }

    #[test]
    fn run_argument_u64_exposes_value() {
        assert_eq!(RunArgumentU64::new(123), RunArgumentU64::new(123));
    }

    #[test]
    fn input_memory_rejects_empty_bytes() {
        assert_eq!(
            InputMemory::from_bytes(Vec::new()),
            Err(InputMemoryError::Empty)
        );
    }

    #[test]
    fn input_memory_exposes_read_only_bytes() {
        let memory = InputMemory::from_bytes(vec![0x48]).expect("input memory is non-empty");

        assert_eq!(memory.bytes(), &[0x48]);
    }

    #[test]
    #[cfg(not(all(unix, target_arch = "aarch64")))]
    fn run_reports_unsupported_host_on_other_hosts() {
        use crate::{
            run_no_args_u64, run_one_input_memory_ptr, run_one_u64, ExecutableMemoryError,
            InputMemory, RunArgumentU64, RunError,
        };

        assert_eq!(
            run_no_args_u64(&[0]),
            Err(RunError::ExecutableMemory(
                ExecutableMemoryError::UnsupportedHost
            ))
        );
        assert_eq!(
            run_one_u64(&[0], RunArgumentU64::new(123)),
            Err(RunError::ExecutableMemory(
                ExecutableMemoryError::UnsupportedHost
            ))
        );
        assert_eq!(
            run_one_input_memory_ptr(
                &[0],
                InputMemory::from_bytes(vec![0x48]).expect("input memory is non-empty")
            ),
            Err(RunError::ExecutableMemory(
                ExecutableMemoryError::UnsupportedHost
            ))
        );
    }
}
