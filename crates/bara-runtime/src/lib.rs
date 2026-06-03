pub mod executable_memory;
pub mod runner;

pub use executable_memory::{ExecutableMemory, ExecutableMemoryError};
pub use runner::{run_no_args_u64, run_one_u64, RunArgumentU64, RunError, RunResult};
