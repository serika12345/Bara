use std::{env, process::ExitCode};

use bara_arm64::emit_program;
use bara_ir::X86Va;
use bara_isa_x86::{decode_function, lift_decoded_function, X86Bytes};
use bara_runtime::run_no_args_u64;

fn main() -> ExitCode {
    match run_cli(env::args().skip(1).collect()) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli(args: Vec<String>) -> Result<String, CliError> {
    match args.as_slice() {
        [command] if command == "check-m1" => run_m1_check(),
        _ => Err(CliError::Usage),
    }
}

fn run_m1_check() -> Result<String, CliError> {
    let input = X86Bytes::new(X86Va::new(0), vec![0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3])
        .map_err(CliError::DecodeInput)?;
    let decoded = decode_function(&input).map_err(CliError::Decode)?;
    let program = lift_decoded_function(&decoded).map_err(CliError::Lift)?;
    let emitted = emit_program(&program).map_err(CliError::Emit)?;
    let result = run_no_args_u64(emitted.code().bytes()).map_err(CliError::Run)?;

    if result.return_value() != 42 {
        return Err(CliError::WrongReturnValue {
            expected: 42,
            actual: result.return_value(),
        });
    }

    Ok(format!(
        "{{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":{},\"stdout\":\"\",\"stderr\":\"\"}}",
        result.return_value()
    ))
}

#[derive(Debug)]
enum CliError {
    Usage,
    DecodeInput(bara_isa_x86::DecodeError),
    Decode(bara_isa_x86::DecodeError),
    Lift(bara_isa_x86::LiftError),
    Emit(bara_arm64::EmitError),
    Run(bara_runtime::RunError),
    WrongReturnValue { expected: u64, actual: u64 },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage => write!(formatter, "usage: btbc-cli check-m1"),
            Self::DecodeInput(error) => write!(formatter, "decode input error: {error:?}"),
            Self::Decode(error) => write!(formatter, "decode error: {error:?}"),
            Self::Lift(error) => write!(formatter, "lift error: {error:?}"),
            Self::Emit(error) => write!(formatter, "emit error: {error:?}"),
            Self::Run(error) => write!(formatter, "run error: {error:?}"),
            Self::WrongReturnValue { expected, actual } => {
                write!(
                    formatter,
                    "wrong return value: expected {expected}, actual {actual}"
                )
            }
        }
    }
}
