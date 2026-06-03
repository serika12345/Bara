pub mod executable_memory;
pub mod runner;

pub use executable_memory::{ExecutableMemory, ExecutableMemoryError};
pub use runner::{run_no_args_u64, RunError, RunResult};
