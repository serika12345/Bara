use bara_arm64::emit_program;
use bara_ir::X86Va;
use bara_isa_x86::{decode_function, lift_decoded_function, X86Bytes};
use bara_runtime::{run_no_args_u64, RunError};

#[test]
fn return_42_decodes_lifts_and_emits_arm64() {
    let input = X86Bytes::new(X86Va::new(0), vec![0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3])
        .expect("self-authored M1 bytes are non-empty");
    let decoded = decode_function(&input).expect("M1 bytes decode");
    let program = lift_decoded_function(&decoded).expect("M1 bytes lift");
    let emitted = emit_program(&program).expect("M1 IR emits");

    assert_eq!(
        emitted.code().bytes(),
        &[0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
    );
    assert_eq!(emitted.pc_map()[0].source(), X86Va::new(0));
}

#[test]
fn return_42_runs_on_supported_aarch64_unix_hosts() {
    let input = X86Bytes::new(X86Va::new(0), vec![0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3])
        .expect("self-authored M1 bytes are non-empty");
    let decoded = decode_function(&input).expect("M1 bytes decode");
    let program = lift_decoded_function(&decoded).expect("M1 bytes lift");
    let emitted = emit_program(&program).expect("M1 IR emits");

    match run_no_args_u64(emitted.code().bytes()) {
        Ok(result) => assert_eq!(result.return_value(), 42),
        Err(RunError::ExecutableMemory(error)) if cfg!(not(all(unix, target_arch = "aarch64"))) => {
            assert_eq!(error, bara_runtime::ExecutableMemoryError::UnsupportedHost);
        }
        Err(error) => panic!("unexpected run error: {error:?}"),
    }
}
