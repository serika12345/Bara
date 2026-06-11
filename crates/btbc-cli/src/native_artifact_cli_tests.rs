use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_mach_o::{
    plan_mach_o_arm64_executable, serialize_mach_o_arm64_executable, MachOArm64ConstData,
    MachOArm64ExecutableWriterRequest, MachOArm64MainCode,
};
use bara_oracle::{
    binary_format_probe_report_to_json, observed_result_from_json, observed_result_to_json,
    probe_public_binary_format, BinaryFileBytes, BinaryFormatProbeError, BinaryInput, FailureKind,
    MachOEntryFunctionTestCaseError, MachOExecutableImageConversionBlocker,
    MachOExecutableImagePlanError,
};

use super::{run_cli, CliError};

#[test]
fn emit_fixture_arm64_writes_return_42_machine_code_file() {
    let temp_dir = TestTempDir::new("emit_fixture_arm64_writes_return_42_machine_code_file");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/return_42.json"),
    );
    let output_path = temp_dir.path.join("return_42.arm64.bin");

    let output = run_cli(vec![
        String::from("emit-fixture-arm64"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("return_42 fixture emits standalone ARM64 machine code");

    assert_eq!(
        read_binary_file(&output_path),
        vec![0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6]
    );
    assert!(output.contains("wrote ARM64 machine code for return_42"));
}

#[test]
fn emit_fixture_arm64_rejects_stdout_host_trap_fixture() {
    let temp_dir = TestTempDir::new("emit_fixture_arm64_rejects_stdout_host_trap_fixture");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/stdout_trap_return_0.json"),
    );
    let output_path = temp_dir.path.join("stdout_trap.arm64.bin");

    let error = run_cli(vec![
        String::from("emit-fixture-arm64"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("stdout host trap fixture is rejected for standalone ARM64 output");

    assert!(matches!(
        error,
        CliError::FunctionRun(super::function_run::FunctionRunError::StandaloneArtifact(_))
    ));
    assert_eq!(error.failure_kind(), FailureKind::EmitError);
    assert!(!output_path.exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn link_fixture_arm64_main_writes_return_42_executable() {
    let temp_dir = TestTempDir::new("link_fixture_arm64_main_writes_return_42_executable");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/return_42.json"),
    );
    let output_path = temp_dir.path.join("return_42");

    let output = run_cli(vec![
        String::from("link-fixture-arm64-main"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("return_42 fixture links as an ARM64 main executable");

    assert!(output_path.exists());
    assert_eq!(
        output,
        format!(
            "{{\"artifact_kind\":\"linked_executable\",\"target_triple\":\"arm64-apple-macos\",\"toolchain\":\"clang\",\"output_path\":\"{}\",\"helper_requirements\":[]}}",
            output_path.display()
        )
    );
    let status = Command::new(&output_path)
        .status()
        .expect("linked executable runs");
    assert_eq!(status.code(), Some(42));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn link_mach_o_arm64_main_writes_return_42_executable() {
    let temp_dir = TestTempDir::new("link_mach_o_arm64_main_writes_return_42_executable");
    let binary_path = temp_dir.write_binary_file(
        "mach_o_return_42.bin",
        include_bytes!("../../../tests/binaries/mach_o_return_42.bin"),
    );
    let output_path = temp_dir.path.join("mach_o_return_42");

    let output = run_cli(vec![
        String::from("link-mach-o-arm64-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("Mach-O backed return_42 links as an ARM64 main executable");

    assert!(output_path.exists());
    assert_eq!(
        output,
        format!(
            "{{\"artifact_kind\":\"linked_executable\",\"target_triple\":\"arm64-apple-macos\",\"toolchain\":\"clang\",\"output_path\":\"{}\",\"helper_requirements\":[],\"source_image\":{{\"kind\":\"mach_o_executable\",\"entry_point\":{{\"entryoff\":130,\"stacksize\":8192}},\"segment\":{{\"name\":\"__TEXT\",\"vmaddr\":4294967296,\"fileoff\":128,\"filesize\":8}}}}}}",
            output_path.display()
        )
    );
    let status = Command::new(&output_path)
        .status()
        .expect("linked executable runs");
    assert_eq!(status.code(), Some(42));
}

#[test]
fn mach_o_stdout_input_reaches_pure_writer_serialization_plan() {
    let temp_dir = TestTempDir::new("mach_o_stdout_input_reaches_pure_writer_serialization_plan");
    let binary_path = temp_dir.write_binary_file(
        "mach_o_hello_world_stdout.bin",
        include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
    );
    let input = super::read_mach_o_artifact_input_with_embedded_host_traps(&binary_path)
        .expect("Mach-O stdout fixture builds artifact input");
    let stdout = input
        .entry_function
        .test_case()
        .host_trap_plan()
        .stdout_trap()
        .expect("embedded metadata provides stdout text");
    let compiled = super::compile_mach_o_entry_function(&input.entry_function)
        .expect("Mach-O stdout entry function compiles");
    let main =
        MachOArm64MainCode::from_emitted_code_bytes(compiled.arm64_bytes().as_slice().to_vec())
            .expect("compiled ARM64 main bytes are non-empty");
    let const_data =
        MachOArm64ConstData::from_read_only_section_bytes(stdout.text().as_bytes().to_vec())
            .expect("stdout const data is non-empty");
    let writer_plan = plan_mach_o_arm64_executable(
        MachOArm64ExecutableWriterRequest::main_with_const_data(main, const_data),
    );

    let serialized = serialize_mach_o_arm64_executable(&writer_plan)
        .expect("pure writer serializes Mach-O-derived payload");
    let layout = serialized.layout();
    let const_section = layout
        .const_section()
        .expect("stdout payload is represented as const data");

    assert_eq!(layout.text_section().offset().value(), 288);
    assert_eq!(
        layout.text_section().size().value(),
        compiled.arm64_bytes().as_slice().len() as u64
    );
    assert_eq!(
        const_section.offset().value(),
        layout.text_section().offset().value() + layout.text_section().size().value()
    );
    assert_eq!(const_section.size().value(), 12);
    assert_eq!(
        serialized
            .bytes_at(layout.text_section())
            .expect("text range is in serialized bytes"),
        compiled.arm64_bytes().as_slice()
    );
    assert_eq!(
        serialized
            .bytes_at(const_section)
            .expect("const range is in serialized bytes"),
        stdout.text().as_bytes()
    );

    let output_probe_input = BinaryInput::from_file_bytes(
        BinaryFileBytes::from_untrusted_file_contents(Vec::from(serialized.bytes())),
    );
    let output_probe = probe_public_binary_format(&output_probe_input)
        .expect("serialized writer output probes as public Mach-O");
    let output_probe_json =
        binary_format_probe_report_to_json(&output_probe).expect("probe report serializes");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&output_probe_json)
            .expect("probe report output is json"),
        serde_json::json!({
            "format": "mach_o_64_little_endian",
            "status": "recognized_but_unsupported",
            "metadata": {
                "mach_o": {
                    "file_type": "executable",
                    "load_commands": {
                        "count": 2,
                        "byte_size": layout.load_commands().size().value(),
                        "recognized_entry_points": [
                            {
                                "byte_size": 24,
                                "entryoff": layout.text_section().offset().value(),
                                "stacksize": 0
                            }
                        ],
                        "recognized_segments": [
                            {
                                "byte_size": layout.load_commands().size().value() - 24,
                                "name": "__TEXT",
                                "vmaddr": 4_294_967_296_u64,
                                "fileoff": 0,
                                "filesize": layout.total_size().value(),
                                "sections": [
                                    {
                                        "name": "__text",
                                        "segment_name": "__TEXT",
                                        "addr": 4_294_967_296_u64
                                            + layout.text_section().offset().value(),
                                        "size": layout.text_section().size().value(),
                                        "offset": layout.text_section().offset().value(),
                                        "align": 2,
                                        "reloff": 0,
                                        "nreloc": 0,
                                        "flags": 2_147_484_672_u32
                                    },
                                    {
                                        "name": "__const",
                                        "segment_name": "__TEXT",
                                        "addr": 4_294_967_296_u64 + const_section.offset().value(),
                                        "size": const_section.size().value(),
                                        "offset": const_section.offset().value(),
                                        "align": 0,
                                        "reloff": 0,
                                        "nreloc": 0,
                                        "flags": 0
                                    }
                                ]
                            }
                        ],
                        "unsupported_commands": []
                    },
                    "executable_image_conversion": {
                        "status": "convertible",
                        "entry_point": {
                            "byte_size": 24,
                            "entryoff": layout.text_section().offset().value(),
                            "stacksize": 0
                        },
                        "segment": {
                            "byte_size": layout.load_commands().size().value() - 24,
                            "name": "__TEXT",
                            "vmaddr": 4_294_967_296_u64,
                            "fileoff": 0,
                            "filesize": layout.total_size().value(),
                            "sections": [
                                {
                                    "name": "__text",
                                    "segment_name": "__TEXT",
                                    "addr": 4_294_967_296_u64
                                        + layout.text_section().offset().value(),
                                    "size": layout.text_section().size().value(),
                                    "offset": layout.text_section().offset().value(),
                                    "align": 2,
                                    "reloff": 0,
                                    "nreloc": 0,
                                    "flags": 2_147_484_672_u32
                                },
                                {
                                    "name": "__const",
                                    "segment_name": "__TEXT",
                                    "addr": 4_294_967_296_u64 + const_section.offset().value(),
                                    "size": const_section.size().value(),
                                    "offset": const_section.offset().value(),
                                    "align": 0,
                                    "reloff": 0,
                                    "nreloc": 0,
                                    "flags": 0
                                }
                            ]
                        }
                    }
                }
            }
        })
    );
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn link_fixture_arm64_main_writes_nested_call_return_42_executable() {
    let temp_dir =
        TestTempDir::new("link_fixture_arm64_main_writes_nested_call_return_42_executable");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/nested_call_return_42.json"),
    );
    let output_path = temp_dir.path.join("nested_call_return_42");

    run_cli(vec![
        String::from("link-fixture-arm64-main"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("nested call fixture links as an ARM64 main executable");

    let status = Command::new(&output_path)
        .status()
        .expect("linked executable runs");
    assert_eq!(status.code(), Some(42));
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[test]
fn link_fixture_arm64_main_reports_unsupported_host() {
    let temp_dir = TestTempDir::new("link_fixture_arm64_main_reports_unsupported_host");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/return_42.json"),
    );
    let output_path = temp_dir.path.join("return_42");

    let error = run_cli(vec![
        String::from("link-fixture-arm64-main"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("non-macOS ARM64 host is unsupported");

    assert!(matches!(
        error,
        CliError::NativeArtifact(
            super::native_artifact::NativeArtifactError::UnsupportedHost { .. }
        )
    ));
    assert_eq!(error.failure_kind(), FailureKind::EmitError);
    assert!(!output_path.exists());
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[test]
fn link_mach_o_arm64_main_reports_unsupported_host() {
    let temp_dir = TestTempDir::new("link_mach_o_arm64_main_reports_unsupported_host");
    let binary_path = temp_dir.write_binary_file(
        "mach_o_return_42.bin",
        include_bytes!("../../../tests/binaries/mach_o_return_42.bin"),
    );
    let output_path = temp_dir.path.join("mach_o_return_42");

    let error = run_cli(vec![
        String::from("link-mach-o-arm64-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("non-macOS ARM64 host is unsupported");

    assert!(matches!(
        error,
        CliError::NativeArtifact(
            super::native_artifact::NativeArtifactError::UnsupportedHost { .. }
        )
    ));
    assert_eq!(error.failure_kind(), FailureKind::EmitError);
    assert!(!output_path.exists());
}

#[test]
fn link_mach_o_arm64_main_preserves_malformed_mach_o_probe_classification() {
    let temp_dir =
        TestTempDir::new("link_mach_o_arm64_main_preserves_malformed_mach_o_probe_classification");
    let binary_path = temp_dir.write_binary_file("short_mach_o.bin", &[0xcf, 0xfa, 0xed]);
    let output_path = temp_dir.path.join("short_mach_o");

    let error = run_cli(vec![
        String::from("link-mach-o-arm64-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("malformed Mach-O is rejected before native linking");

    assert!(matches!(
        error,
        CliError::MachOEntryFunctionTestCase(MachOEntryFunctionTestCaseError::Probe(
            BinaryFormatProbeError::InputTooShort
        ))
    ));
    assert_eq!(error.failure_kind(), FailureKind::InvalidTestCase);
    assert!(!output_path.exists());
}

#[test]
fn link_mach_o_arm64_main_preserves_unsupported_mach_o_blocker_classification() {
    let temp_dir = TestTempDir::new(
        "link_mach_o_arm64_main_preserves_unsupported_mach_o_blocker_classification",
    );
    let binary_path =
        temp_dir.write_binary_file("mach_o_missing_segment.bin", MACH_O_MISSING_SEGMENT_BIN);
    let output_path = temp_dir.path.join("mach_o_missing_segment");

    let error = run_cli(vec![
        String::from("link-mach-o-arm64-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("unsupported Mach-O is rejected before native linking");

    assert_missing_segment_blocker_error(error, &output_path);
}

#[test]
fn link_fixture_arm64_main_rejects_stdout_host_trap_fixture() {
    let temp_dir = TestTempDir::new("link_fixture_arm64_main_rejects_stdout_host_trap_fixture");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/stdout_trap_return_0.json"),
    );
    let output_path = temp_dir.path.join("stdout_trap");

    let error = run_cli(vec![
        String::from("link-fixture-arm64-main"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("stdout host trap fixture is rejected before native linking");

    assert!(matches!(
        error,
        CliError::FunctionRun(super::function_run::FunctionRunError::StandaloneArtifact(_))
    ));
    assert_eq!(error.failure_kind(), FailureKind::EmitError);
    assert!(!output_path.exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn link_fixture_arm64_stdout_main_writes_hello_world_executable() {
    let temp_dir = TestTempDir::new("link_fixture_arm64_stdout_main_writes_hello_world_executable");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/hello_world_stdout_return_0.json"),
    );
    let output_path = temp_dir.path.join("hello_world_stdout");

    let output = run_cli(vec![
        String::from("link-fixture-arm64-stdout-main"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("hello_world_stdout fixture links as an ARM64 stdout main executable");

    assert!(output_path.exists());
    let expected = observed_result_from_json(include_str!(
        "../../../tests/expected/hello_world_stdout_return_0.json"
    ))
    .and_then(|result| observed_result_to_json(&result))
    .expect("expected stdout fixture normalizes to output json");
    assert_eq!(output, expected);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn link_mach_o_arm64_stdout_main_writes_hello_world_executable() {
    let temp_dir = TestTempDir::new("link_mach_o_arm64_stdout_main_writes_hello_world_executable");
    let binary_path = temp_dir.write_binary_file(
        "mach_o_hello_world_stdout.bin",
        include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
    );
    let output_path = temp_dir.path.join("mach_o_hello_world_stdout");

    let output = run_cli(vec![
        String::from("link-mach-o-arm64-stdout-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("Mach-O backed hello world links as an ARM64 stdout main executable");

    assert!(output_path.exists());
    let expected = observed_result_from_json(include_str!(
        "../../../tests/expected/mach_o_hello_world_stdout.json"
    ))
    .and_then(|result| observed_result_to_json(&result))
    .expect("expected Mach-O stdout fixture normalizes to output json");
    assert_eq!(output, expected);
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[test]
fn link_fixture_arm64_stdout_main_reports_unsupported_host() {
    let temp_dir = TestTempDir::new("link_fixture_arm64_stdout_main_reports_unsupported_host");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/hello_world_stdout_return_0.json"),
    );
    let output_path = temp_dir.path.join("hello_world_stdout");

    let error = run_cli(vec![
        String::from("link-fixture-arm64-stdout-main"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("non-macOS ARM64 host is unsupported");

    assert!(matches!(
        error,
        CliError::NativeArtifact(
            super::native_artifact::NativeArtifactError::UnsupportedHost { .. }
        )
    ));
    assert_eq!(error.failure_kind(), FailureKind::EmitError);
    assert!(!output_path.exists());
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[test]
fn link_mach_o_arm64_stdout_main_reports_unsupported_host() {
    let temp_dir = TestTempDir::new("link_mach_o_arm64_stdout_main_reports_unsupported_host");
    let binary_path = temp_dir.write_binary_file(
        "mach_o_hello_world_stdout.bin",
        include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
    );
    let output_path = temp_dir.path.join("mach_o_hello_world_stdout");

    let error = run_cli(vec![
        String::from("link-mach-o-arm64-stdout-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("non-macOS ARM64 host is unsupported");

    assert!(matches!(
        error,
        CliError::NativeArtifact(
            super::native_artifact::NativeArtifactError::UnsupportedHost { .. }
        )
    ));
    assert_eq!(error.failure_kind(), FailureKind::EmitError);
    assert!(!output_path.exists());
}

#[test]
fn link_mach_o_arm64_stdout_main_preserves_malformed_mach_o_probe_classification() {
    let temp_dir = TestTempDir::new(
        "link_mach_o_arm64_stdout_main_preserves_malformed_mach_o_probe_classification",
    );
    let binary_path = temp_dir.write_binary_file("short_mach_o.bin", &[0xcf, 0xfa, 0xed]);
    let output_path = temp_dir.path.join("short_mach_o_stdout");

    let error = run_cli(vec![
        String::from("link-mach-o-arm64-stdout-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("malformed Mach-O is rejected before native stdout linking");

    assert!(matches!(
        error,
        CliError::MachOEntryFunctionTestCase(MachOEntryFunctionTestCaseError::Probe(
            BinaryFormatProbeError::InputTooShort
        ))
    ));
    assert_eq!(error.failure_kind(), FailureKind::InvalidTestCase);
    assert!(!output_path.exists());
}

#[test]
fn link_mach_o_arm64_stdout_main_preserves_unsupported_mach_o_blocker_classification() {
    let temp_dir = TestTempDir::new(
        "link_mach_o_arm64_stdout_main_preserves_unsupported_mach_o_blocker_classification",
    );
    let binary_path =
        temp_dir.write_binary_file("mach_o_missing_segment.bin", MACH_O_MISSING_SEGMENT_BIN);
    let output_path = temp_dir.path.join("mach_o_missing_segment_stdout");

    let error = run_cli(vec![
        String::from("link-mach-o-arm64-stdout-main"),
        binary_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("unsupported Mach-O is rejected before native stdout linking");

    assert_missing_segment_blocker_error(error, &output_path);
}

#[test]
fn link_fixture_arm64_stdout_main_rejects_fixture_without_stdout_request() {
    let temp_dir =
        TestTempDir::new("link_fixture_arm64_stdout_main_rejects_fixture_without_stdout_request");
    let case_path = temp_dir.write_file(
        "case.json",
        include_str!("../../../tests/cases/return_42.json"),
    );
    let output_path = temp_dir.path.join("return_42_stdout");

    let error = run_cli(vec![
        String::from("link-fixture-arm64-stdout-main"),
        case_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect_err("return_42 fixture has no stdout request");

    assert!(matches!(
        error,
        CliError::NativeArtifact(
            super::native_artifact::NativeArtifactError::StdoutMainUnsupported(
                super::native_artifact::NativeStdoutMainUnsupported::MissingStdoutTrapPlan
            )
        )
    ));
    assert_eq!(error.failure_kind(), FailureKind::EmitError);
    assert!(!output_path.exists());
}

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("bara-{name}-{nanos}"));
        fs::create_dir(&path).expect("test temp dir is created");
        Self { path }
    }

    fn write_file(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("test fixture file is written");
        path
    }

    fn write_binary_file(&self, name: &str, contents: &[u8]) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("test fixture binary file is written");
        path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("test temp dir is removed");
    }
}

const MACH_O_MISSING_SEGMENT_BIN: &[u8] = &[
    0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x28, 0x00, 0x00, 0x80, 0x18, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn assert_missing_segment_blocker_error(error: CliError, output_path: &Path) {
    assert!(matches!(
        error,
        CliError::MachOEntryFunctionTestCase(MachOEntryFunctionTestCaseError::Plan(
            MachOExecutableImagePlanError::NotConvertible {
                blocker: MachOExecutableImageConversionBlocker::MissingSegment
            }
        ))
    ));
    assert_eq!(error.failure_kind(), FailureKind::InvalidTestCase);
    assert!(!output_path.exists());
}

fn read_binary_file(path: &Path) -> Vec<u8> {
    fs::read(path).expect("test fixture binary file is read")
}
