use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use bara_oracle::{observed_result_from_json, observed_result_to_json, FailureKind};

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
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("test temp dir is removed");
    }
}

fn read_binary_file(path: &Path) -> Vec<u8> {
    fs::read(path).expect("test fixture binary file is read")
}
