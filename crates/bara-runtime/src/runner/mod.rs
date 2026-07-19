use crate::{ExecutableMemory, ExecutableMemoryError, HostTrapPlan, RunStdout};
use bara_arm64::TranslationArtifact;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResult {
    return_value: u64,
    stdout: RunStdout,
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
        Self {
            return_value,
            stdout: RunStdout::empty(),
        }
    }

    const fn with_stdout(return_value: u64, stdout: RunStdout) -> Self {
        Self {
            return_value,
            stdout,
        }
    }

    pub const fn return_value(&self) -> u64 {
        self.return_value
    }

    pub fn stdout(&self) -> &str {
        self.stdout.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunError {
    ExecutableMemory(ExecutableMemoryError),
    UnsupportedHost,
}

pub fn run_translation_artifact_no_args_u64(
    artifact: &TranslationArtifact,
) -> Result<RunResult, RunError> {
    run_no_args_u64(artifact.emitted_function().code().bytes())
}

pub fn run_translation_artifact_no_args_u64_with_host_traps(
    artifact: &TranslationArtifact,
    host_traps: HostTrapPlan,
) -> Result<RunResult, RunError> {
    run_no_args_u64_with_host_traps(artifact.emitted_function().code().bytes(), host_traps)
}

pub fn run_translation_artifact_one_u64(
    artifact: &TranslationArtifact,
    argument: RunArgumentU64,
) -> Result<RunResult, RunError> {
    run_one_u64(artifact.emitted_function().code().bytes(), argument)
}

pub fn run_translation_artifact_one_input_memory_ptr(
    artifact: &TranslationArtifact,
    memory: InputMemory,
) -> Result<RunResult, RunError> {
    run_one_input_memory_ptr(artifact.emitted_function().code().bytes(), memory)
}

pub fn run_no_args_u64(code: &[u8]) -> Result<RunResult, RunError> {
    run_no_args_u64_with_host_traps(code, HostTrapPlan::none())
}

pub fn run_no_args_u64_with_host_traps(
    code: &[u8],
    host_traps: HostTrapPlan,
) -> Result<RunResult, RunError> {
    let executable = ExecutableMemory::allocate(code).map_err(RunError::ExecutableMemory)?;
    let result = call_no_args_u64(&executable)?;
    Ok(RunResult::with_stdout(
        result.return_value(),
        host_traps.stdout_output().clone(),
    ))
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
    use std::str::FromStr;

    use bara_arm64::{
        Arm64MachineCode, EmittedFunction, TranslationArtifact, TranslationCacheIdentity,
        TranslationSourceHash, TranslationSourceIdentity, TranslationTarget, TranslatorVersion,
    };

    use crate::{InputMemory, InputMemoryError, RunArgumentU64, RunError, RunResult, RunStdout};

    #[test]
    fn run_result_exposes_return_value() {
        assert_eq!(RunResult::new(42).return_value(), 42);
    }

    #[test]
    fn run_result_exposes_stdout() {
        let result = RunResult::with_stdout(
            0,
            RunStdout::from_text(String::from("hello trap\n")).expect("stdout trap text is ascii"),
        );

        assert_eq!(result.stdout(), "hello trap\n");
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
    fn translation_artifact_runner_preserves_supported_abi_results() {
        let artifact = return_42_artifact();

        assert_run_result(crate::run_translation_artifact_no_args_u64(&artifact));
        assert_run_result(crate::run_translation_artifact_one_u64(
            &artifact,
            RunArgumentU64::new(123),
        ));
        assert_run_result(crate::run_translation_artifact_one_input_memory_ptr(
            &artifact,
            InputMemory::from_bytes(vec![0x48]).expect("input memory is non-empty"),
        ));
    }

    #[test]
    fn translation_artifact_runner_preserves_host_trap_output() {
        let artifact = return_42_artifact();
        let host_traps = crate::HostTrapPlan::stdout(
            RunStdout::from_text(String::from("hello artifact\n"))
                .expect("stdout trap text is ascii"),
        );
        let result =
            crate::run_translation_artifact_no_args_u64_with_host_traps(&artifact, host_traps);

        if cfg!(all(unix, target_arch = "aarch64")) {
            let result = result.expect("supported host should execute the artifact");
            assert_eq!(result.return_value(), 42);
            assert_eq!(result.stdout(), "hello artifact\n");
        } else {
            assert_eq!(
                result,
                Err(RunError::ExecutableMemory(
                    crate::ExecutableMemoryError::UnsupportedHost
                ))
            );
        }
    }

    #[test]
    #[cfg(not(all(unix, target_arch = "aarch64")))]
    fn run_reports_unsupported_host_on_other_hosts() {
        use crate::{
            run_no_args_u64, run_one_input_memory_ptr, run_one_u64, ExecutableMemoryError,
            HostTrapPlan, InputMemory, RunArgumentU64, RunError,
        };

        assert_eq!(
            run_no_args_u64(&[0]),
            Err(RunError::ExecutableMemory(
                ExecutableMemoryError::UnsupportedHost
            ))
        );
        assert_eq!(
            crate::run_no_args_u64_with_host_traps(&[0], HostTrapPlan::none()),
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

    fn assert_run_result(result: Result<RunResult, RunError>) {
        if cfg!(all(unix, target_arch = "aarch64")) {
            assert_eq!(
                result
                    .expect("supported host should execute the artifact")
                    .return_value(),
                42
            );
        } else {
            assert_eq!(
                result,
                Err(RunError::ExecutableMemory(
                    crate::ExecutableMemoryError::UnsupportedHost
                ))
            );
        }
    }

    fn return_42_artifact() -> TranslationArtifact {
        const SOURCE_HASH: &str =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let source_hash =
            TranslationSourceHash::from_str(SOURCE_HASH).expect("test source hash should be valid");

        TranslationArtifact::new(
            TranslationSourceIdentity::new(source_hash),
            EmittedFunction::new(
                Arm64MachineCode::new(vec![0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6])
                    .expect("test ARM64 machine code should be valid"),
                Vec::new(),
            ),
            TranslationCacheIdentity::new(
                source_hash,
                TranslatorVersion::from_str("0.1.0")
                    .expect("test translator version should be valid"),
                TranslationTarget::Arm64MacOs,
            ),
        )
        .expect("matching source and cache identities should construct an artifact")
    }
}
