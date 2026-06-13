use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
};

use bara_oracle::{
    binary_format_probe_report_from_json, binary_format_probe_report_to_json,
    compare_observed_results, corpus_report_to_json, executable_manifest_from_json,
    host_trap_plan_from_json, mach_o_entry_function_input_with_embedded_host_traps,
    mach_o_entry_function_input_with_host_traps, mach_o_entry_function_test_case,
    mach_o_entry_function_test_case_with_embedded_host_traps,
    mach_o_entry_function_test_case_with_host_traps, observed_result_from_json,
    observed_result_to_json, probe_public_binary_format, test_case_from_json, BinaryFileBytes,
    BinaryFormatProbeError, BinaryFormatProbeReport, BinaryInput, CaseId, ComparisonIssue,
    ComparisonReport, CorpusReport, ExecutableManifest, ExpectedResult, FailureKind,
    FailureMessage, FixtureOutcome, FixtureReport, JsonError, MachOEntryFunctionInput,
    MachOEntryFunctionTestCaseError, ObservedResult, TestCase,
};
use serde::Serialize;

mod b8_debug_bundle;
mod blackbox_run;
mod executable_run;
mod function_run;
mod gui_hello_world_actual;
mod gui_hello_world_translated;
mod native_artifact;
#[cfg(test)]
mod native_artifact_cli_tests;
mod x86_64_mach_o_fixture;

use b8_debug_bundle::{generate_b8_debug_bundle, B8DebugBundleError};
use blackbox_run::run_check_blackbox;
use executable_run::{run_executable_manifest, ExecutableRunError};
use function_run::{
    compile_mach_o_entry_function, compile_mach_o_entry_function_standalone_artifact,
    compile_test_case_function, compile_test_case_function_standalone_artifact,
    run_test_case_function, FunctionArtifactMetadata, FunctionRunError,
};
use gui_hello_world_actual::{
    b8_gui_hello_world_actual_launch_attempt, b8_gui_hello_world_feedback_report,
};
use gui_hello_world_translated::{
    b8_gui_hello_world_translated_launch, GuiHelloWorldTranslatedLaunchError,
    GuiHelloWorldTranslatedLaunchMode,
};
use native_artifact::{
    link_arm64_main_executable, link_arm64_main_executable_with_source_metadata,
    link_arm64_stdout_main_executable, link_arm64_stdout_main_executable_with_source_metadata,
    native_artifact_metadata_to_json, observe_native_executable_artifact, NativeArtifactError,
    NativeSourceImageMetadata, NativeSourceImageMetadataError,
};
use x86_64_mach_o_fixture::{
    build_x86_64_gui_hello_world_fixture, build_x86_64_gui_hello_world_manual_visible_fixture,
    build_x86_64_mach_o_fixture, build_x86_64_oracle_runner,
    observe_appkit_gui_hello_world_helper_actual,
    observe_appkit_gui_hello_world_manual_visible_helper_actual,
    observe_x86_64_gui_hello_world_expected, observe_x86_64_oracle_expected,
    X8664MachOFixtureError,
};

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
        [command, case_path, expected_path] if command == "check-fixture" => {
            run_check_fixture(Path::new(case_path), Path::new(expected_path))
        }
        [command, manifest_path, expected_path] if command == "check-executable" => {
            run_check_executable(Path::new(manifest_path), Path::new(expected_path))
        }
        [command, binary_path, expected_path] if command == "check-mach-o" => {
            run_check_mach_o(Path::new(binary_path), Path::new(expected_path))
        }
        [command, binary_path, host_traps_path, expected_path]
            if command == "check-mach-o-host-traps" =>
        {
            run_check_mach_o_host_traps(
                Path::new(binary_path),
                Path::new(host_traps_path),
                Path::new(expected_path),
            )
        }
        [command, binary_path, expected_path] if command == "check-mach-o-host-traps" => {
            run_check_mach_o_embedded_host_traps(Path::new(binary_path), Path::new(expected_path))
        }
        [command, binary_path] if command == "probe-binary" => {
            run_probe_binary(Path::new(binary_path))
        }
        [command, binary_path, expected_path] if command == "check-binary-probe" => {
            run_check_binary_probe(Path::new(binary_path), Path::new(expected_path))
        }
        [command, case_path, output_path] if command == "emit-fixture-arm64" => {
            run_emit_fixture_arm64(Path::new(case_path), Path::new(output_path))
        }
        [command, case_path, output_path] if command == "link-fixture-arm64-main" => {
            run_link_fixture_arm64_main(Path::new(case_path), Path::new(output_path))
        }
        [command, case_path, output_path] if command == "build-x86_64-macho-fixture" => {
            run_build_x86_64_mach_o_fixture(Path::new(case_path), Path::new(output_path))
        }
        [command, output_path] if command == "build-x86_64-gui-hello-world-fixture" => {
            run_build_x86_64_gui_hello_world_fixture(Path::new(output_path))
        }
        [command, output_path] if command == "build-x86_64-gui-hello-world-visible-fixture" => {
            run_build_x86_64_gui_hello_world_visible_fixture(Path::new(output_path))
        }
        [command, case_path, output_path] if command == "build-x86_64-oracle-runner" => {
            run_build_x86_64_oracle_runner(Path::new(case_path), Path::new(output_path))
        }
        [command, case_path, expected_path] if command == "generate-x86_64-expected" => {
            run_generate_x86_64_expected(Path::new(case_path), Path::new(expected_path))
        }
        [command, expected_path, launch_metadata_path]
            if command == "generate-x86_64-gui-hello-world-expected" =>
        {
            run_generate_x86_64_gui_hello_world_expected(
                Path::new(expected_path),
                Path::new(launch_metadata_path),
            )
        }
        [command, case_path, actual_path] if command == "generate-arm64-actual" => {
            run_generate_arm64_actual(Path::new(case_path), Path::new(actual_path))
        }
        [command, binary_path, actual_path, launch_report_path]
            if command == "generate-arm64-gui-hello-world-actual" =>
        {
            run_generate_arm64_gui_hello_world_actual(
                Path::new(binary_path),
                Path::new(actual_path),
                Path::new(launch_report_path),
            )
        }
        [command, binary_path, actual_path, launch_report_path]
            if command == "generate-arm64-gui-hello-world-translated-actual" =>
        {
            run_generate_arm64_gui_hello_world_translated_actual(
                Path::new(binary_path),
                Path::new(actual_path),
                Path::new(launch_report_path),
            )
        }
        [command, binary_path, launch_report_path]
            if command == "run-arm64-gui-hello-world-translated-visible" =>
        {
            run_arm64_gui_hello_world_translated_visible(
                Path::new(binary_path),
                Path::new(launch_report_path),
            )
        }
        [command, binary_path, expected_path, actual_path, launch_report_path, feedback_report_path]
            if command == "generate-arm64-gui-hello-world-feedback" =>
        {
            run_generate_arm64_gui_hello_world_feedback(
                Path::new(binary_path),
                Path::new(expected_path),
                Path::new(actual_path),
                Path::new(launch_report_path),
                Path::new(feedback_report_path),
            )
        }
        [command, binary_path, output_root] if command == "generate-b8-debug-bundle" => {
            run_generate_b8_debug_bundle(Path::new(binary_path), Path::new(output_root))
        }
        [command, case_path, output_dir] if command == "emit-fixture-artifacts" => {
            run_emit_fixture_artifacts(Path::new(case_path), Path::new(output_dir))
        }
        [command, expected_path, actual_path] if command == "compare-expected-actual" => {
            run_compare_expected_actual(Path::new(expected_path), Path::new(actual_path))
        }
        [command, binary_path, output_path] if command == "link-mach-o-arm64-main" => {
            run_link_mach_o_arm64_main(Path::new(binary_path), Path::new(output_path))
        }
        [command, case_path, output_path] if command == "link-fixture-arm64-stdout-main" => {
            run_link_fixture_arm64_stdout_main(Path::new(case_path), Path::new(output_path))
        }
        [command, binary_path, host_traps_path, output_path]
            if command == "link-mach-o-arm64-stdout-main" =>
        {
            run_link_mach_o_arm64_stdout_main_with_host_traps(
                Path::new(binary_path),
                Path::new(host_traps_path),
                Path::new(output_path),
            )
        }
        [command, binary_path, output_path] if command == "link-mach-o-arm64-stdout-main" => {
            run_link_mach_o_arm64_stdout_main(Path::new(binary_path), Path::new(output_path))
        }
        [command] if command == "check-blackbox" => run_check_blackbox(None),
        [command, output_flag, output_dir]
            if command == "check-blackbox" && output_flag == "--out" =>
        {
            run_check_blackbox(Some(Path::new(output_dir)))
        }
        [command, cases_dir, expected_dir] if command == "check-corpus" => {
            run_check_corpus(Path::new(cases_dir), Path::new(expected_dir))
        }
        [command, cases_dir, expected_dir, output_flag, output_dir]
            if command == "check-corpus" && output_flag == "--out" =>
        {
            run_check_corpus_with_output(
                Path::new(cases_dir),
                Path::new(expected_dir),
                Some(Path::new(output_dir)),
            )
        }
        _ => Err(CliError::Usage),
    }
}

fn run_m1_check() -> Result<String, CliError> {
    run_m1_check_from_fixtures(
        include_str!("../../../tests/cases/return_42.json"),
        include_str!("../../../tests/expected/return_42.json"),
    )
}

fn run_m1_check_from_fixtures(case_json: &str, expected_json: &str) -> Result<String, CliError> {
    let test_case = test_case_from_json(case_json).map_err(CliError::TestCase)?;
    let expected = observed_result_from_json(expected_json).map_err(CliError::ExpectedJson)?;
    run_test_case(test_case, expected)
}

fn run_check_fixture(case_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let expected_json = read_text_file(expected_path)?;

    run_m1_check_from_fixtures(&case_json, &expected_json)
}

fn run_emit_fixture_arm64(case_path: &Path, output_path: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let compiled = compile_test_case_function_standalone_artifact(&test_case)
        .map_err(CliError::FunctionRun)?;
    write_binary_file(output_path, compiled.arm64_bytes().as_slice())?;

    Ok(format!(
        "wrote ARM64 machine code for {} to {}",
        test_case.case_id().as_str(),
        output_path.display()
    ))
}

fn run_link_fixture_arm64_main(case_path: &Path, output_path: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let compiled = compile_test_case_function_standalone_artifact(&test_case)
        .map_err(CliError::FunctionRun)?;
    let artifact = link_arm64_main_executable(compiled.arm64_bytes(), output_path)
        .map_err(CliError::NativeArtifact)?;

    native_artifact_metadata_to_json(artifact.metadata()).map_err(CliError::Json)
}

fn run_build_x86_64_mach_o_fixture(
    case_path: &Path,
    output_path: &Path,
) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let fixture = build_x86_64_mach_o_fixture(&test_case, output_path)
        .map_err(CliError::X8664MachOFixture)?;

    serde_json::to_string(fixture.metadata())
        .map_err(JsonError::new)
        .map_err(CliError::Json)
}

fn run_build_x86_64_gui_hello_world_fixture(output_path: &Path) -> Result<String, CliError> {
    let fixture =
        build_x86_64_gui_hello_world_fixture(output_path).map_err(CliError::X8664MachOFixture)?;

    serde_json::to_string(fixture.metadata())
        .map_err(JsonError::new)
        .map_err(CliError::Json)
}

fn run_build_x86_64_gui_hello_world_visible_fixture(
    output_path: &Path,
) -> Result<String, CliError> {
    let fixture = build_x86_64_gui_hello_world_manual_visible_fixture(output_path)
        .map_err(CliError::X8664MachOFixture)?;

    serde_json::to_string(fixture.metadata())
        .map_err(JsonError::new)
        .map_err(CliError::Json)
}

fn run_build_x86_64_oracle_runner(
    case_path: &Path,
    output_path: &Path,
) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let runner =
        build_x86_64_oracle_runner(&test_case, output_path).map_err(CliError::X8664MachOFixture)?;

    serde_json::to_string(runner.metadata())
        .map_err(JsonError::new)
        .map_err(CliError::Json)
}

fn run_generate_x86_64_expected(
    case_path: &Path,
    expected_path: &Path,
) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let expected =
        observe_x86_64_oracle_expected(&test_case).map_err(CliError::X8664MachOFixture)?;
    let expected_json = observed_result_to_json(&expected).map_err(CliError::Json)?;
    create_output_parent_dir(expected_path)?;
    write_text_file(expected_path, &expected_json)?;

    Ok(expected_json)
}

fn run_generate_x86_64_gui_hello_world_expected(
    expected_path: &Path,
    launch_metadata_path: &Path,
) -> Result<String, CliError> {
    let expected_bundle =
        observe_x86_64_gui_hello_world_expected().map_err(CliError::X8664MachOFixture)?;
    let expected_json =
        observed_result_to_json(expected_bundle.observed_result()).map_err(CliError::Json)?;
    let launch_metadata_json = serde_json::to_string(expected_bundle.launch_metadata())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;

    create_output_parent_dir(expected_path)?;
    write_text_file(expected_path, &expected_json)?;
    create_output_parent_dir(launch_metadata_path)?;
    write_text_file(launch_metadata_path, &launch_metadata_json)?;

    serde_json::to_string(&GuiHelloWorldExpectedOutputPaths::new(
        expected_path,
        launch_metadata_path,
    ))
    .map_err(JsonError::new)
    .map_err(CliError::Json)
}

#[derive(Serialize)]
struct GuiHelloWorldExpectedOutputPaths {
    expected: String,
    launch_metadata: String,
}

impl GuiHelloWorldExpectedOutputPaths {
    fn new(expected_path: &Path, launch_metadata_path: &Path) -> Self {
        Self {
            expected: expected_path.to_string_lossy().into_owned(),
            launch_metadata: launch_metadata_path.to_string_lossy().into_owned(),
        }
    }
}

fn run_generate_arm64_actual(case_path: &Path, actual_path: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let actual = observe_test_case(&test_case)?;
    let actual_json = observed_result_to_json(&actual).map_err(CliError::Json)?;
    create_output_parent_dir(actual_path)?;
    write_text_file(actual_path, &actual_json)?;

    Ok(actual_json)
}

fn run_generate_arm64_gui_hello_world_actual(
    binary_path: &Path,
    actual_path: &Path,
    launch_report_path: &Path,
) -> Result<String, CliError> {
    let attempt = run_b8_gui_hello_world_actual_attempt(binary_path)?;
    let actual_json = observed_result_to_json(attempt.observed_result()).map_err(CliError::Json)?;
    let launch_report_json = serde_json::to_string(attempt.launch_report())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;

    create_output_parent_dir(actual_path)?;
    write_text_file(actual_path, &actual_json)?;
    create_output_parent_dir(launch_report_path)?;
    write_text_file(launch_report_path, &launch_report_json)?;

    serde_json::to_string(&GuiHelloWorldActualOutputPaths::new(
        actual_path,
        launch_report_path,
    ))
    .map_err(JsonError::new)
    .map_err(CliError::Json)
}

fn run_generate_arm64_gui_hello_world_feedback(
    binary_path: &Path,
    expected_path: &Path,
    actual_path: &Path,
    launch_report_path: &Path,
    feedback_report_path: &Path,
) -> Result<String, CliError> {
    let expected_json = read_text_file(expected_path)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
    let attempt = run_b8_gui_hello_world_actual_attempt(binary_path)?;
    let actual_json = observed_result_to_json(attempt.observed_result()).map_err(CliError::Json)?;
    let launch_report_json = serde_json::to_string(attempt.launch_report())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;
    let feedback_report = b8_gui_hello_world_feedback_report(&expected, &attempt);
    let feedback_report_json = serde_json::to_string(&feedback_report)
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;

    create_output_parent_dir(actual_path)?;
    write_text_file(actual_path, &actual_json)?;
    create_output_parent_dir(launch_report_path)?;
    write_text_file(launch_report_path, &launch_report_json)?;
    create_output_parent_dir(feedback_report_path)?;
    write_text_file(feedback_report_path, &feedback_report_json)?;

    serde_json::to_string(&GuiHelloWorldFeedbackOutputPaths::new(
        actual_path,
        launch_report_path,
        feedback_report_path,
    ))
    .map_err(JsonError::new)
    .map_err(CliError::Json)
}

fn run_generate_arm64_gui_hello_world_translated_actual(
    binary_path: &Path,
    actual_path: &Path,
    launch_report_path: &Path,
) -> Result<String, CliError> {
    let input_probe = probe_binary_path(binary_path)?;
    let helper_result =
        observe_appkit_gui_hello_world_helper_actual().map_err(CliError::X8664MachOFixture)?;
    let launch = b8_gui_hello_world_translated_launch(
        input_probe,
        helper_result,
        GuiHelloWorldTranslatedLaunchMode::AutomatedOracle,
    )
    .map_err(CliError::GuiHelloWorldTranslatedLaunch)?;
    let actual_json = observed_result_to_json(launch.observed_result()).map_err(CliError::Json)?;
    let launch_report_json = serde_json::to_string(launch.launch_report())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;

    create_output_parent_dir(actual_path)?;
    write_text_file(actual_path, &actual_json)?;
    create_output_parent_dir(launch_report_path)?;
    write_text_file(launch_report_path, &launch_report_json)?;

    serde_json::to_string(&GuiHelloWorldActualOutputPaths::new(
        actual_path,
        launch_report_path,
    ))
    .map_err(JsonError::new)
    .map_err(CliError::Json)
}

fn run_arm64_gui_hello_world_translated_visible(
    binary_path: &Path,
    launch_report_path: &Path,
) -> Result<String, CliError> {
    let input_probe = probe_binary_path(binary_path)?;
    let helper_result = observe_appkit_gui_hello_world_manual_visible_helper_actual()
        .map_err(CliError::X8664MachOFixture)?;
    let launch = b8_gui_hello_world_translated_launch(
        input_probe,
        helper_result,
        GuiHelloWorldTranslatedLaunchMode::ManualVisible,
    )
    .map_err(CliError::GuiHelloWorldTranslatedLaunch)?;
    let launch_report_json = serde_json::to_string(launch.launch_report())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;

    create_output_parent_dir(launch_report_path)?;
    write_text_file(launch_report_path, &launch_report_json)?;

    serde_json::to_string(&GuiHelloWorldVisibleOutputPaths::new(launch_report_path))
        .map_err(JsonError::new)
        .map_err(CliError::Json)
}

fn run_b8_gui_hello_world_actual_attempt(
    binary_path: &Path,
) -> Result<gui_hello_world_actual::GuiHelloWorldActualLaunchBundle, CliError> {
    let input_probe = probe_binary_path(binary_path)?;
    let helper_result =
        observe_appkit_gui_hello_world_helper_actual().map_err(CliError::X8664MachOFixture)?;
    b8_gui_hello_world_actual_launch_attempt(&input_probe, helper_result)
        .map_err(CliError::X8664MachOFixture)
}

fn run_generate_b8_debug_bundle(
    binary_path: &Path,
    output_root: &Path,
) -> Result<String, CliError> {
    generate_b8_debug_bundle(binary_path, output_root).map_err(CliError::B8DebugBundle)
}

fn probe_binary_path(binary_path: &Path) -> Result<BinaryFormatProbeReport, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    probe_public_binary_format(&input).map_err(CliError::BinaryFormatProbe)
}

#[derive(Serialize)]
struct GuiHelloWorldActualOutputPaths {
    actual: String,
    launch_report: String,
}

impl GuiHelloWorldActualOutputPaths {
    fn new(actual_path: &Path, launch_report_path: &Path) -> Self {
        Self {
            actual: actual_path.to_string_lossy().into_owned(),
            launch_report: launch_report_path.to_string_lossy().into_owned(),
        }
    }
}

#[derive(Serialize)]
struct GuiHelloWorldVisibleOutputPaths {
    launch_report: String,
}

impl GuiHelloWorldVisibleOutputPaths {
    fn new(launch_report_path: &Path) -> Self {
        Self {
            launch_report: launch_report_path.to_string_lossy().into_owned(),
        }
    }
}

#[derive(Serialize)]
struct GuiHelloWorldFeedbackOutputPaths {
    actual: String,
    launch_report: String,
    feedback_report: String,
}

impl GuiHelloWorldFeedbackOutputPaths {
    fn new(actual_path: &Path, launch_report_path: &Path, feedback_report_path: &Path) -> Self {
        Self {
            actual: actual_path.to_string_lossy().into_owned(),
            launch_report: launch_report_path.to_string_lossy().into_owned(),
            feedback_report: feedback_report_path.to_string_lossy().into_owned(),
        }
    }
}

fn run_emit_fixture_artifacts(case_path: &Path, output_dir: &Path) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let compiled = compile_test_case_function(&test_case).map_err(CliError::FunctionRun)?;
    let artifacts = compiled.artifact_metadata(&test_case);
    let output_paths = write_fixture_artifacts(output_dir, &artifacts)?;

    serde_json::to_string(&output_paths)
        .map_err(JsonError::new)
        .map_err(CliError::Json)
}

fn write_fixture_artifacts(
    output_dir: &Path,
    artifacts: &FunctionArtifactMetadata,
) -> Result<FixtureArtifactOutputPaths, CliError> {
    let output_paths = FixtureArtifactOutputPaths::from_dir(output_dir);
    let compiled_ir_json = serde_json::to_string(artifacts.compiled_ir())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;
    let pcmap_json = serde_json::to_string(artifacts.pcmap())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;
    let fixups_json = serde_json::to_string(artifacts.fixups())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;
    let helpers_json = serde_json::to_string(artifacts.helpers())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;
    let artifact_report_json = serde_json::to_string(artifacts.artifact_report())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;
    let verifier_report_json = serde_json::to_string(artifacts.verifier_report())
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;

    create_dir(output_dir)?;
    write_text_file(&output_paths.compiled_ir_path(), &compiled_ir_json)?;
    write_text_file(&output_paths.pcmap_path(), &pcmap_json)?;
    write_text_file(&output_paths.fixups_path(), &fixups_json)?;
    write_text_file(&output_paths.helpers_path(), &helpers_json)?;
    write_text_file(&output_paths.artifact_report_path(), &artifact_report_json)?;
    write_text_file(&output_paths.verifier_report_path(), &verifier_report_json)?;

    Ok(output_paths)
}

fn run_compare_expected_actual(
    expected_path: &Path,
    actual_path: &Path,
) -> Result<String, CliError> {
    let expected_json = read_text_file(expected_path)?;
    let actual_json = read_text_file(actual_path)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
    let actual = observed_result_from_json(&actual_json).map_err(CliError::ExpectedJson)?;
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        return Err(CliError::Comparison(comparison));
    }

    serde_json::to_string(&comparison)
        .map_err(JsonError::new)
        .map_err(CliError::Json)
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct FixtureArtifactOutputPaths {
    compiled_ir: String,
    pcmap: String,
    fixups: String,
    helpers: String,
    artifact_report: String,
    verifier_report: String,
}

impl FixtureArtifactOutputPaths {
    fn from_dir(output_dir: &Path) -> Self {
        Self {
            compiled_ir: output_dir
                .join("compiled.ir.json")
                .to_string_lossy()
                .into_owned(),
            pcmap: output_dir.join("pcmap.json").to_string_lossy().into_owned(),
            fixups: output_dir
                .join("fixups.json")
                .to_string_lossy()
                .into_owned(),
            helpers: output_dir
                .join("helpers.json")
                .to_string_lossy()
                .into_owned(),
            artifact_report: output_dir
                .join("artifact.report.json")
                .to_string_lossy()
                .into_owned(),
            verifier_report: output_dir
                .join("verifier.report.json")
                .to_string_lossy()
                .into_owned(),
        }
    }

    fn compiled_ir_path(&self) -> PathBuf {
        PathBuf::from(&self.compiled_ir)
    }

    fn pcmap_path(&self) -> PathBuf {
        PathBuf::from(&self.pcmap)
    }

    fn fixups_path(&self) -> PathBuf {
        PathBuf::from(&self.fixups)
    }

    fn helpers_path(&self) -> PathBuf {
        PathBuf::from(&self.helpers)
    }

    fn artifact_report_path(&self) -> PathBuf {
        PathBuf::from(&self.artifact_report)
    }

    fn verifier_report_path(&self) -> PathBuf {
        PathBuf::from(&self.verifier_report)
    }
}

fn run_link_mach_o_arm64_main(binary_path: &Path, output_path: &Path) -> Result<String, CliError> {
    let input = read_mach_o_artifact_input(binary_path)?;
    let compiled = compile_mach_o_entry_function_standalone_artifact(&input.entry_function)
        .map_err(CliError::FunctionRun)?;
    let artifact = link_arm64_main_executable_with_source_metadata(
        compiled.arm64_bytes(),
        output_path,
        Some(input.source_image),
    )
    .map_err(CliError::NativeArtifact)?;

    native_artifact_metadata_to_json(artifact.metadata()).map_err(CliError::Json)
}

fn run_link_fixture_arm64_stdout_main(
    case_path: &Path,
    output_path: &Path,
) -> Result<String, CliError> {
    let case_json = read_text_file(case_path)?;
    let test_case = test_case_from_json(&case_json).map_err(CliError::TestCase)?;
    let compiled = compile_test_case_function(&test_case).map_err(CliError::FunctionRun)?;
    let artifact = link_arm64_stdout_main_executable(
        compiled.arm64_bytes(),
        test_case.host_trap_plan(),
        compiled.stdout_host_trap_request(),
        output_path,
    )
    .map_err(CliError::NativeArtifact)?;

    let actual = observe_native_executable_artifact(test_case.case_id().clone(), &artifact)
        .map_err(CliError::NativeArtifact)?;
    observed_result_to_json(&actual).map_err(CliError::Json)
}

fn run_link_mach_o_arm64_stdout_main(
    binary_path: &Path,
    output_path: &Path,
) -> Result<String, CliError> {
    link_mach_o_arm64_stdout_main_from_input(
        read_mach_o_artifact_input_with_embedded_host_traps(binary_path)?,
        output_path,
    )
}

fn run_link_mach_o_arm64_stdout_main_with_host_traps(
    binary_path: &Path,
    host_traps_path: &Path,
    output_path: &Path,
) -> Result<String, CliError> {
    let input = read_mach_o_artifact_input_with_host_traps(
        binary_path,
        read_host_trap_plan(host_traps_path)?,
    )?;
    link_mach_o_arm64_stdout_main_from_input(input, output_path)
}

fn link_mach_o_arm64_stdout_main_from_input(
    input: MachOArtifactInput,
    output_path: &Path,
) -> Result<String, CliError> {
    let MachOArtifactInput {
        entry_function,
        source_image,
    } = input;
    let test_case = entry_function.test_case();
    let compiled = compile_mach_o_entry_function(&entry_function).map_err(CliError::FunctionRun)?;
    let artifact = link_arm64_stdout_main_executable_with_source_metadata(
        compiled.arm64_bytes(),
        test_case.host_trap_plan(),
        compiled.stdout_host_trap_request(),
        output_path,
        Some(source_image),
    )
    .map_err(CliError::NativeArtifact)?;

    let actual = observe_native_executable_artifact(test_case.case_id().clone(), &artifact)
        .map_err(CliError::NativeArtifact)?;
    observed_result_to_json(&actual).map_err(CliError::Json)
}

fn run_check_executable(manifest_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let manifest_json = read_text_file(manifest_path)?;
    let expected_json = read_text_file(expected_path)?;
    let manifest =
        executable_manifest_from_json(&manifest_json).map_err(CliError::ExecutableManifest)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;

    run_executable(manifest, expected)
}

fn run_check_mach_o(binary_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let expected_json = read_text_file(expected_path)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
    let test_case = read_mach_o_entry_function_test_case(binary_path)?;

    run_test_case(test_case, expected)
}

fn run_check_mach_o_host_traps(
    binary_path: &Path,
    host_traps_path: &Path,
    expected_path: &Path,
) -> Result<String, CliError> {
    let expected_json = read_text_file(expected_path)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
    let test_case = read_mach_o_entry_function_test_case_with_host_traps(
        binary_path,
        read_host_trap_plan(host_traps_path)?,
    )?;

    run_test_case(test_case, expected)
}

fn run_check_mach_o_embedded_host_traps(
    binary_path: &Path,
    expected_path: &Path,
) -> Result<String, CliError> {
    let expected_json = read_text_file(expected_path)?;
    let expected = observed_result_from_json(&expected_json).map_err(CliError::ExpectedJson)?;
    let test_case = read_mach_o_entry_function_test_case_with_embedded_host_traps(binary_path)?;

    run_test_case(test_case, expected)
}

fn read_mach_o_entry_function_test_case(binary_path: &Path) -> Result<TestCase, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    mach_o_entry_function_test_case(case_id_from_path(binary_path), &input)
        .map_err(CliError::MachOEntryFunctionTestCase)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MachOArtifactInput {
    entry_function: MachOEntryFunctionInput,
    source_image: NativeSourceImageMetadata,
}

fn read_mach_o_artifact_input(binary_path: &Path) -> Result<MachOArtifactInput, CliError> {
    read_mach_o_artifact_input_with_host_traps(
        binary_path,
        bara_oracle::TestCaseHostTrapPlan::none(),
    )
}

fn read_mach_o_artifact_input_with_embedded_host_traps(
    binary_path: &Path,
) -> Result<MachOArtifactInput, CliError> {
    read_mach_o_artifact_input_from_binary(binary_path, |case_id, input| {
        mach_o_entry_function_input_with_embedded_host_traps(case_id, input)
    })
}

fn read_mach_o_entry_function_test_case_with_host_traps(
    binary_path: &Path,
    host_trap_plan: bara_oracle::TestCaseHostTrapPlan,
) -> Result<TestCase, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    mach_o_entry_function_test_case_with_host_traps(
        case_id_from_path(binary_path),
        &input,
        host_trap_plan,
    )
    .map_err(CliError::MachOEntryFunctionTestCase)
}

fn read_mach_o_entry_function_test_case_with_embedded_host_traps(
    binary_path: &Path,
) -> Result<TestCase, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    mach_o_entry_function_test_case_with_embedded_host_traps(case_id_from_path(binary_path), &input)
        .map_err(CliError::MachOEntryFunctionTestCase)
}

fn read_mach_o_artifact_input_with_host_traps(
    binary_path: &Path,
    host_trap_plan: bara_oracle::TestCaseHostTrapPlan,
) -> Result<MachOArtifactInput, CliError> {
    read_mach_o_artifact_input_from_binary(binary_path, |case_id, input| {
        mach_o_entry_function_input_with_host_traps(case_id, input, host_trap_plan)
    })
}

fn read_mach_o_artifact_input_from_binary(
    binary_path: &Path,
    entry_function_from_input: impl FnOnce(
        CaseId,
        &BinaryInput,
    ) -> Result<
        MachOEntryFunctionInput,
        MachOEntryFunctionTestCaseError,
    >,
) -> Result<MachOArtifactInput, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    let entry_function = entry_function_from_input(case_id_from_path(binary_path), &input)
        .map_err(CliError::MachOEntryFunctionTestCase)?;
    let report = probe_public_binary_format(&input).map_err(CliError::BinaryFormatProbe)?;
    let source_image = NativeSourceImageMetadata::from_mach_o_conversion(
        report
            .metadata()
            .mach_o_metadata()
            .executable_image_conversion(),
    )
    .map_err(CliError::NativeSourceImageMetadata)?;

    Ok(MachOArtifactInput {
        entry_function,
        source_image,
    })
}

fn read_host_trap_plan(
    host_traps_path: &Path,
) -> Result<bara_oracle::TestCaseHostTrapPlan, CliError> {
    let host_traps_json = read_text_file(host_traps_path)?;
    host_trap_plan_from_json(&host_traps_json).map_err(CliError::HostTrapPlan)
}

fn run_probe_binary(binary_path: &Path) -> Result<String, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    let report = probe_public_binary_format(&input).map_err(CliError::BinaryFormatProbe)?;

    binary_format_probe_report_to_json(&report).map_err(CliError::Json)
}

fn run_check_binary_probe(binary_path: &Path, expected_path: &Path) -> Result<String, CliError> {
    let bytes = read_binary_file(binary_path)?;
    let expected_json = read_text_file(expected_path)?;
    let expected = binary_format_probe_report_from_json(&expected_json)
        .map_err(CliError::ExpectedProbeJson)?;
    let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(bytes));
    let actual = probe_public_binary_format(&input).map_err(CliError::BinaryFormatProbe)?;

    if actual != expected {
        return Err(CliError::BinaryProbeComparisonMismatch {
            expected: Box::new(expected),
            actual: Box::new(actual),
        });
    }

    binary_format_probe_report_to_json(&actual).map_err(CliError::Json)
}

fn run_check_corpus(cases_dir: &Path, expected_dir: &Path) -> Result<String, CliError> {
    run_check_corpus_with_output(cases_dir, expected_dir, None)
}

fn run_check_corpus_with_output(
    cases_dir: &Path,
    expected_dir: &Path,
    output_dir: Option<&Path>,
) -> Result<String, CliError> {
    let case_paths = sorted_case_paths(cases_dir)?;
    if case_paths.is_empty() {
        return Err(CliError::EmptyCorpus {
            cases_dir: cases_dir.to_path_buf(),
        });
    }

    let mut fixture_runs = Vec::new();
    for case_path in case_paths {
        fixture_runs.push(run_corpus_fixture(&case_path, expected_dir));
    }

    let report = fixture_runs
        .iter()
        .map(|run| run.report.clone())
        .collect::<CorpusReport>();
    if let Some(output_dir) = output_dir {
        write_corpus_outputs(output_dir, &report, &fixture_runs)?;
    }
    if !report.is_success() {
        return Err(CliError::CorpusFailures(report));
    }

    corpus_report_to_json(&report).map_err(CliError::Json)
}

fn run_corpus_fixture(case_path: &Path, expected_dir: &Path) -> FixtureRun {
    let fallback_case_id = case_id_from_path(case_path);
    let case_json = match read_text_file(case_path) {
        Ok(case_json) => case_json,
        Err(error) => {
            return FixtureRun::failed(
                fallback_case_id,
                FailureKind::InvalidTestCase,
                error.to_string(),
            );
        }
    };
    let test_case = match test_case_from_json(&case_json) {
        Ok(test_case) => test_case,
        Err(error) => {
            return FixtureRun::failed(
                fallback_case_id,
                FailureKind::InvalidTestCase,
                error.to_string(),
            )
            .with_failure_artifacts(FixtureFailureArtifacts::new(
                Some(case_json),
                None,
                None,
            ));
        }
    };
    let case_id = test_case.case_id().clone();
    let expected_path = expected_dir.join(format!("{}.json", case_id.as_str()));
    let expected_json = match read_text_file(&expected_path) {
        Ok(expected_json) => expected_json,
        Err(error) => {
            return FixtureRun::failed(case_id, FailureKind::MissingExpected, error.to_string())
                .with_failure_artifacts(FixtureFailureArtifacts::new(Some(case_json), None, None));
        }
    };
    let expected = match observed_result_from_json(&expected_json) {
        Ok(expected) => expected,
        Err(error) => {
            return FixtureRun::failed(case_id, FailureKind::InvalidExpected, error.to_string())
                .with_failure_artifacts(FixtureFailureArtifacts::new(
                    Some(case_json),
                    Some(expected_json),
                    None,
                ));
        }
    };

    let actual = match observe_test_case(&test_case) {
        Ok(actual) => actual,
        Err(error) => {
            return FixtureRun::failed(case_id, error.failure_kind(), error.to_string())
                .with_failure_artifacts(FixtureFailureArtifacts::new(
                    Some(case_json),
                    Some(expected_json),
                    None,
                ));
        }
    };
    let artifact_metadata = match fixture_artifact_metadata(&test_case) {
        Ok(artifact_metadata) => artifact_metadata,
        Err(error) => {
            return FixtureRun::failed_with_actual(
                case_id,
                error.failure_kind(),
                error.to_string(),
                actual.clone(),
            )
            .with_failure_artifacts(FixtureFailureArtifacts::new(
                Some(case_json),
                Some(expected_json),
                Some(actual),
            ));
        }
    };
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        let message = format!("comparison failed: {comparison:?}");
        return FixtureRun::failed_with_actual_and_artifacts(
            case_id,
            failure_kind_from_comparison_report(&comparison),
            message,
            actual.clone(),
            artifact_metadata,
        )
        .with_final_state_report(comparison)
        .with_failure_artifacts(FixtureFailureArtifacts::new(
            Some(case_json),
            Some(expected_json),
            Some(actual),
        ));
    }

    FixtureRun::passed_observed_with_artifacts(case_id, actual, artifact_metadata)
}

fn run_test_case(test_case: TestCase, expected: ExpectedResult) -> Result<String, CliError> {
    let actual = observe_test_case(&test_case)?;
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        return Err(CliError::Comparison(comparison));
    }

    observed_result_to_json(&actual).map_err(CliError::Json)
}

fn observe_test_case(test_case: &TestCase) -> Result<ObservedResult, CliError> {
    run_test_case_function(test_case)
        .map_err(CliError::FunctionRun)
        .map(|result| result.into_observed_result(test_case.case_id().clone()))
}

fn fixture_artifact_metadata(test_case: &TestCase) -> Result<FunctionArtifactMetadata, CliError> {
    compile_test_case_function(test_case)
        .map_err(CliError::FunctionRun)
        .map(|compiled| compiled.artifact_metadata(test_case))
}

fn run_executable(
    manifest: ExecutableManifest,
    expected: ExpectedResult,
) -> Result<String, CliError> {
    let actual = run_executable_manifest(&manifest)
        .map_err(CliError::ExecutableRun)?
        .into_observed_result();
    let comparison = compare_observed_results(&expected, &actual);
    if !comparison.is_match() {
        return Err(CliError::Comparison(comparison));
    }

    observed_result_to_json(&actual).map_err(CliError::Json)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FixtureRun {
    report: FixtureReport,
    output: Option<FixtureOutput>,
    artifact_metadata: Option<FunctionArtifactMetadata>,
    final_state_report: Option<ComparisonReport>,
    failure_artifacts: Option<FixtureFailureArtifacts>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FixtureOutput {
    Observed(ObservedResult),
    Probe(Box<BinaryFormatProbeReport>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FixtureFailureArtifacts {
    testcase_json: Option<String>,
    expected_json: Option<String>,
    actual: Option<ObservedResult>,
}

impl FixtureFailureArtifacts {
    const fn new(
        testcase_json: Option<String>,
        expected_json: Option<String>,
        actual: Option<ObservedResult>,
    ) -> Self {
        Self {
            testcase_json,
            expected_json,
            actual,
        }
    }
}

impl FixtureRun {
    fn passed_observed(case_id: CaseId, actual: ObservedResult) -> Self {
        Self {
            report: FixtureReport::new(case_id, FixtureOutcome::Passed),
            output: Some(FixtureOutput::Observed(actual)),
            artifact_metadata: None,
            final_state_report: None,
            failure_artifacts: None,
        }
    }

    fn passed_observed_with_artifacts(
        case_id: CaseId,
        actual: ObservedResult,
        artifact_metadata: FunctionArtifactMetadata,
    ) -> Self {
        Self {
            report: FixtureReport::new(case_id, FixtureOutcome::Passed),
            output: Some(FixtureOutput::Observed(actual)),
            artifact_metadata: Some(artifact_metadata),
            final_state_report: None,
            failure_artifacts: None,
        }
    }

    fn passed_probe(case_id: CaseId, actual: BinaryFormatProbeReport) -> Self {
        Self {
            report: FixtureReport::new(case_id, FixtureOutcome::Passed),
            output: Some(FixtureOutput::Probe(Box::new(actual))),
            artifact_metadata: None,
            final_state_report: None,
            failure_artifacts: None,
        }
    }

    fn failed(case_id: CaseId, kind: FailureKind, message: String) -> Self {
        Self {
            report: failed_fixture_report(case_id, kind, message),
            output: None,
            artifact_metadata: None,
            final_state_report: None,
            failure_artifacts: None,
        }
    }

    fn failed_with_actual(
        case_id: CaseId,
        kind: FailureKind,
        message: String,
        actual: ObservedResult,
    ) -> Self {
        Self {
            report: failed_fixture_report(case_id, kind, message),
            output: Some(FixtureOutput::Observed(actual)),
            artifact_metadata: None,
            final_state_report: None,
            failure_artifacts: None,
        }
    }

    fn failed_with_actual_and_artifacts(
        case_id: CaseId,
        kind: FailureKind,
        message: String,
        actual: ObservedResult,
        artifact_metadata: FunctionArtifactMetadata,
    ) -> Self {
        Self {
            report: failed_fixture_report(case_id, kind, message),
            output: Some(FixtureOutput::Observed(actual)),
            artifact_metadata: Some(artifact_metadata),
            final_state_report: None,
            failure_artifacts: None,
        }
    }

    fn with_final_state_report(mut self, final_state_report: ComparisonReport) -> Self {
        self.final_state_report = Some(final_state_report);
        self
    }

    fn with_failure_artifacts(mut self, failure_artifacts: FixtureFailureArtifacts) -> Self {
        self.failure_artifacts = Some(failure_artifacts);
        self
    }
}

fn failed_fixture_report(case_id: CaseId, kind: FailureKind, message: String) -> FixtureReport {
    FixtureReport::new(
        case_id,
        FixtureOutcome::failed(kind, FailureMessage::from(message)),
    )
}

fn write_corpus_outputs(
    output_dir: &Path,
    report: &CorpusReport,
    fixture_runs: &[FixtureRun],
) -> Result<(), CliError> {
    create_dir(output_dir)?;
    let actual_dir = output_dir.join("actual");
    let compiled_dir = output_dir.join("compiled");
    create_dir(&actual_dir)?;
    create_dir(&compiled_dir)?;

    let report_json = corpus_report_to_json(report).map_err(CliError::Json)?;
    write_text_file(&output_dir.join("report.json"), &report_json)?;

    for run in fixture_runs {
        if let FixtureOutcome::Failed { kind, message } = run.report.outcome() {
            write_fixture_failure_package(
                &output_dir
                    .join("failures")
                    .join(run.report.case_id().as_str()),
                run.report.case_id(),
                *kind,
                message.as_str(),
                run.final_state_report.as_ref(),
                run.failure_artifacts.as_ref(),
            )?;
        }

        if let Some(artifact_metadata) = &run.artifact_metadata {
            write_fixture_artifacts(
                &compiled_dir.join(run.report.case_id().as_str()),
                artifact_metadata,
            )?;
        }

        if let Some(output) = &run.output {
            let (case_id, actual_json) = match output {
                FixtureOutput::Observed(actual) => (
                    actual.case_id().as_str(),
                    observed_result_to_json(actual).map_err(CliError::Json)?,
                ),
                FixtureOutput::Probe(actual) => (
                    run.report.case_id().as_str(),
                    binary_format_probe_report_to_json(actual).map_err(CliError::Json)?,
                ),
            };
            write_text_file(&actual_dir.join(format!("{case_id}.json")), &actual_json)?;
        }
    }

    Ok(())
}

fn write_fixture_failure_package(
    failure_dir: &Path,
    case_id: &CaseId,
    kind: FailureKind,
    message: &str,
    final_state_report: Option<&ComparisonReport>,
    artifacts: Option<&FixtureFailureArtifacts>,
) -> Result<(), CliError> {
    create_dir(failure_dir)?;
    let report =
        FixtureFailurePackageJson::new(case_id, kind, message, final_state_report, artifacts);
    let report_json = serde_json::to_string(&report)
        .map_err(JsonError::new)
        .map_err(CliError::Json)?;
    write_text_file(&failure_dir.join("failure.json"), &report_json)?;

    if let Some(artifacts) = artifacts {
        if let Some(testcase_json) = &artifacts.testcase_json {
            write_text_file(&failure_dir.join("testcase.json"), testcase_json)?;
        }
        if let Some(expected_json) = &artifacts.expected_json {
            write_text_file(&failure_dir.join("expected.json"), expected_json)?;
        }
        if let Some(actual) = &artifacts.actual {
            let actual_json = observed_result_to_json(actual).map_err(CliError::Json)?;
            write_text_file(&failure_dir.join("actual.json"), &actual_json)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct FixtureFailurePackageJson<'a> {
    case_id: &'a str,
    kind: FailureKind,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    final_state: Option<&'a ComparisonReport>,
    shrink: FixtureFailureShrinkJson,
    corpus_update: FixtureCorpusUpdateJson,
}

impl<'a> FixtureFailurePackageJson<'a> {
    fn new(
        case_id: &'a CaseId,
        kind: FailureKind,
        message: &'a str,
        final_state_report: Option<&'a ComparisonReport>,
        artifacts: Option<&FixtureFailureArtifacts>,
    ) -> Self {
        Self {
            case_id: case_id.as_str(),
            kind,
            message,
            final_state: final_state_report,
            shrink: FixtureFailureShrinkJson {
                status: FixtureShrinkStatus::NotAttempted,
                recommended_next_step: format!(
                    "minimize testcase while preserving failure kind {}",
                    failure_kind_label(kind)
                ),
            },
            corpus_update: FixtureCorpusUpdateJson::from_artifacts(artifacts),
        }
    }
}

#[derive(Serialize)]
struct FixtureFailureShrinkJson {
    status: FixtureShrinkStatus,
    recommended_next_step: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum FixtureShrinkStatus {
    NotAttempted,
}

#[derive(Serialize)]
struct FixtureCorpusUpdateJson {
    action: FixtureCorpusUpdateAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_testcase: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_expected: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_actual: Option<&'static str>,
}

impl FixtureCorpusUpdateJson {
    fn from_artifacts(artifacts: Option<&FixtureFailureArtifacts>) -> Self {
        Self {
            action: FixtureCorpusUpdateAction::ReviewFailurePackage,
            candidate_testcase: artifacts
                .and_then(|artifacts| artifacts.testcase_json.as_ref())
                .map(|_| "testcase.json"),
            candidate_expected: artifacts
                .and_then(|artifacts| artifacts.expected_json.as_ref())
                .map(|_| "expected.json"),
            candidate_actual: artifacts
                .and_then(|artifacts| artifacts.actual.as_ref())
                .map(|_| "actual.json"),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum FixtureCorpusUpdateAction {
    ReviewFailurePackage,
}

fn failure_kind_label(kind: FailureKind) -> &'static str {
    match kind {
        FailureKind::InvalidTestCase => "invalid_test_case",
        FailureKind::MissingExpected => "missing_expected",
        FailureKind::InvalidExpected => "invalid_expected",
        FailureKind::DecodeError => "decode_error",
        FailureKind::LiftError => "lift_error",
        FailureKind::EmitError => "emit_error",
        FailureKind::RunError => "run_error",
        FailureKind::ComparisonMismatch => "comparison_mismatch",
        FailureKind::UnsupportedInstruction => "unsupported_instruction",
        FailureKind::WrongReturnValue => "wrong_return_value",
        FailureKind::WrongRegisterValue => "wrong_register_value",
        FailureKind::WrongFlags => "wrong_flags",
        FailureKind::WrongMemory => "wrong_memory",
        FailureKind::WrongBranchTarget => "wrong_branch_target",
        FailureKind::WrongCallReturn => "wrong_call_return",
        FailureKind::WrongExternalCall => "wrong_external_call",
        FailureKind::RunnerCrash => "runner_crash",
        FailureKind::OracleCrash => "oracle_crash",
    }
}

pub(crate) fn failure_kind_from_comparison_report(report: &ComparisonReport) -> FailureKind {
    if report
        .issues()
        .iter()
        .any(|issue| matches!(issue, ComparisonIssue::ReturnValueMismatch { .. }))
    {
        return FailureKind::WrongRegisterValue;
    }

    if report
        .issues()
        .iter()
        .any(|issue| matches!(issue, ComparisonIssue::StdoutMismatch { .. }))
    {
        return FailureKind::WrongExternalCall;
    }

    if report
        .issues()
        .iter()
        .any(|issue| matches!(issue, ComparisonIssue::ExitStatusMismatch { .. }))
    {
        return FailureKind::WrongCallReturn;
    }

    FailureKind::ComparisonMismatch
}

fn create_dir(path: &Path) -> Result<(), CliError> {
    fs::create_dir_all(path).map_err(|source| CliError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

fn create_output_parent_dir(path: &Path) -> Result<(), CliError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        create_dir(parent)?;
    }

    Ok(())
}

fn read_text_file(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

fn read_binary_file(path: &Path) -> Result<Vec<u8>, CliError> {
    fs::read(path).map_err(|source| CliError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), CliError> {
    fs::write(path, contents).map_err(|source| CliError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn write_binary_file(path: &Path, contents: &[u8]) -> Result<(), CliError> {
    fs::write(path, contents).map_err(|source| CliError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn sorted_case_paths(cases_dir: &Path) -> Result<Vec<PathBuf>, CliError> {
    let mut paths = Vec::new();
    let entries = fs::read_dir(cases_dir).map_err(|source| CliError::ReadDir {
        path: cases_dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| CliError::ReadDirEntry {
            path: cases_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn case_id_from_path(path: &Path) -> CaseId {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| CaseId::new(stem).ok())
        .unwrap_or_else(|| CaseId::new("unknown").expect("fallback case id is non-empty"))
}

#[derive(Debug)]
enum CliError {
    Usage,
    ReadFile {
        path: PathBuf,
        source: io::Error,
    },
    WriteFile {
        path: PathBuf,
        source: io::Error,
    },
    CreateDir {
        path: PathBuf,
        source: io::Error,
    },
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    ReadDirEntry {
        path: PathBuf,
        source: io::Error,
    },
    EmptyCorpus {
        cases_dir: PathBuf,
    },
    ExecutableManifest(bara_oracle::ExecutableManifestJsonError),
    TestCase(bara_oracle::TestCaseJsonError),
    HostTrapPlan(bara_oracle::TestCaseJsonError),
    ExpectedJson(bara_oracle::JsonError),
    ExpectedProbeJson(bara_oracle::JsonError),
    BinaryFormatProbe(BinaryFormatProbeError),
    MachOEntryFunctionTestCase(MachOEntryFunctionTestCaseError),
    BinaryProbeComparisonMismatch {
        expected: Box<BinaryFormatProbeReport>,
        actual: Box<BinaryFormatProbeReport>,
    },
    FunctionRun(FunctionRunError),
    GuiHelloWorldTranslatedLaunch(GuiHelloWorldTranslatedLaunchError),
    B8DebugBundle(B8DebugBundleError),
    NativeArtifact(NativeArtifactError),
    X8664MachOFixture(X8664MachOFixtureError),
    NativeSourceImageMetadata(NativeSourceImageMetadataError),
    ExecutableRun(ExecutableRunError),
    Comparison(ComparisonReport),
    Json(bara_oracle::JsonError),
    CorpusFailures(CorpusReport),
}

impl CliError {
    fn failure_kind(&self) -> FailureKind {
        match self {
            Self::TestCase(_) => FailureKind::InvalidTestCase,
            Self::HostTrapPlan(_) => FailureKind::InvalidTestCase,
            Self::ExecutableManifest(_) => FailureKind::InvalidTestCase,
            Self::ExpectedJson(_) => FailureKind::InvalidExpected,
            Self::ExpectedProbeJson(_) => FailureKind::InvalidExpected,
            Self::BinaryFormatProbe(_) => FailureKind::InvalidTestCase,
            Self::MachOEntryFunctionTestCase(_) => FailureKind::InvalidTestCase,
            Self::BinaryProbeComparisonMismatch { .. } => FailureKind::ComparisonMismatch,
            Self::FunctionRun(error) => error.failure_kind(),
            Self::GuiHelloWorldTranslatedLaunch(_) => FailureKind::RunError,
            Self::B8DebugBundle(_) => FailureKind::RunError,
            Self::NativeArtifact(error) => error.failure_kind(),
            Self::X8664MachOFixture(error) => error.failure_kind(),
            Self::NativeSourceImageMetadata(_) => FailureKind::InvalidTestCase,
            Self::ExecutableRun(error) => error.failure_kind(),
            Self::Comparison(report) => failure_kind_from_comparison_report(report),
            Self::ReadFile { .. } | Self::WriteFile { .. } | Self::CreateDir { .. } => {
                FailureKind::InvalidTestCase
            }
            Self::ReadDir { .. }
            | Self::ReadDirEntry { .. }
            | Self::EmptyCorpus { .. }
            | Self::Usage
            | Self::Json(_)
            | Self::CorpusFailures(_) => FailureKind::InvalidTestCase,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage => write!(
                formatter,
                "usage: btbc-cli check-m1 | check-fixture <case.json> <expected.json> | check-executable <manifest.json> <expected.json> | check-mach-o <binary> <expected.json> | check-mach-o-host-traps <binary> <expected.json> | check-mach-o-host-traps <binary> <host-traps.json> <expected.json> | check-corpus <cases-dir> <expected-dir> [--out <dir>] | probe-binary <path> | check-binary-probe <binary> <expected.json> | emit-fixture-arm64 <case.json> <out.bin> | emit-fixture-artifacts <case.json> <out-dir> | link-fixture-arm64-main <case.json> <out-exe> | build-x86_64-macho-fixture <case.json> <out-exe> | build-x86_64-gui-hello-world-fixture <out-exe> | build-x86_64-gui-hello-world-visible-fixture <out-exe> | build-x86_64-oracle-runner <case.json> <out-exe> | generate-x86_64-expected <case.json> <expected.json> | generate-x86_64-gui-hello-world-expected <expected.json> <launch-metadata.json> | generate-arm64-actual <case.json> <actual.json> | generate-arm64-gui-hello-world-actual <binary> <actual.json> <launch-report.json> | generate-arm64-gui-hello-world-translated-actual <binary> <actual.json> <launch-report.json> | run-arm64-gui-hello-world-translated-visible <binary> <launch-report.json> | generate-arm64-gui-hello-world-feedback <binary> <expected.json> <actual.json> <launch-report.json> <feedback-report.json> | generate-b8-debug-bundle <binary> <out-root> | compare-expected-actual <expected.json> <actual.json> | link-mach-o-arm64-main <binary> <out-exe> | link-fixture-arm64-stdout-main <case.json> <out-exe> | link-mach-o-arm64-stdout-main <binary> <out-exe> | link-mach-o-arm64-stdout-main <binary> <host-traps.json> <out-exe> | check-blackbox [--out <dir>]"
            ),
            Self::ReadFile { path, source } => {
                write!(formatter, "failed to read file {}: {source}", path.display())
            }
            Self::WriteFile { path, source } => {
                write!(
                    formatter,
                    "failed to write file {}: {source}",
                    path.display()
                )
            }
            Self::CreateDir { path, source } => {
                write!(
                    formatter,
                    "failed to create directory {}: {source}",
                    path.display()
                )
            }
            Self::ReadDir { path, source } => {
                write!(
                    formatter,
                    "failed to read directory {}: {source}",
                    path.display()
                )
            }
            Self::ReadDirEntry { path, source } => {
                write!(
                    formatter,
                    "failed to read directory entry under {}: {source}",
                    path.display()
                )
            }
            Self::EmptyCorpus { cases_dir } => {
                write!(
                    formatter,
                    "no testcase json files found in {}",
                    cases_dir.display()
                )
            }
            Self::ExecutableManifest(error) => {
                write!(formatter, "executable manifest error: {error}")
            }
            Self::TestCase(error) => write!(formatter, "testcase error: {error}"),
            Self::HostTrapPlan(error) => write!(formatter, "host trap plan error: {error}"),
            Self::ExpectedJson(error) => write!(formatter, "expected json error: {error}"),
            Self::ExpectedProbeJson(error) => {
                write!(formatter, "expected probe json error: {error}")
            }
            Self::BinaryFormatProbe(error) => {
                write!(formatter, "binary format probe error: {error:?}")
            }
            Self::MachOEntryFunctionTestCase(error) => {
                write!(formatter, "mach-o entry function testcase error: {error:?}")
            }
            Self::BinaryProbeComparisonMismatch { expected, actual } => {
                write!(
                    formatter,
                    "binary probe comparison failed: expected {expected:?}, actual {actual:?}"
                )
            }
            Self::FunctionRun(error) => write!(formatter, "function run error: {error}"),
            Self::GuiHelloWorldTranslatedLaunch(error) => {
                write!(formatter, "B8 translated GUI launch error: {error}")
            }
            Self::B8DebugBundle(error) => write!(formatter, "B8 debug bundle error: {error}"),
            Self::NativeArtifact(error) => write!(formatter, "native artifact error: {error}"),
            Self::X8664MachOFixture(error) => {
                write!(formatter, "x86_64 Mach-O fixture error: {error}")
            }
            Self::NativeSourceImageMetadata(error) => {
                write!(formatter, "native source image metadata error: {error}")
            }
            Self::ExecutableRun(error) => write!(formatter, "executable run error: {error}"),
            Self::Comparison(report) => write!(formatter, "comparison failed: {report:?}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::CorpusFailures(report) => match corpus_report_to_json(report) {
                Ok(json) => write!(formatter, "{json}"),
                Err(error) => write!(formatter, "failed to serialize corpus report: {error}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bara_oracle::{
        binary_format_probe_report_from_json, binary_format_probe_report_to_json,
        observed_result_from_json, observed_result_to_json, FailureKind, FailureMessage,
        FixtureOutcome,
    };

    use super::{run_cli, run_m1_check_from_fixtures, CliError};

    const MACH_O_HELLO_WORLD_STDOUT_HOST_TRAPS_JSON: &str = concat!(
        "{\n",
        "  \"host_traps\": [\n",
        "    {\n",
        "      \"kind\": \"stdout\",\n",
        "      \"text\": \"hello world\\n\"\n",
        "    }\n",
        "  ]\n",
        "}\n",
    );
    const MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON: &str = concat!(
        "{\n",
        "  \"case_id\": \"mach_o_hello_world_stdout\",\n",
        "  \"exit_status\": 0,\n",
        "  \"return_value\": 0,\n",
        "  \"stdout\": \"hello world\\n\",\n",
        "  \"stderr\": \"\"\n",
        "}\n",
    );

    #[test]
    fn unknown_command_reports_usage() {
        assert!(matches!(
            run_cli(vec!["unknown".to_owned()]),
            Err(CliError::Usage)
        ));
    }

    #[test]
    fn no_command_reports_usage() {
        assert!(matches!(run_cli(Vec::new()), Err(CliError::Usage)));
    }

    #[test]
    fn check_m1_matches_return_42_fixtures() {
        let output = run_m1_check_from_fixtures(
            include_str!("../../../tests/cases/return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        )
        .expect("return_42 fixture check succeeds on supported host");
        let expected =
            observed_result_from_json(include_str!("../../../tests/expected/return_42.json"))
                .and_then(|result| observed_result_to_json(&result))
                .expect("expected fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_fixture_reads_case_and_expected_files() {
        let temp_dir = TestTempDir::new("check_fixture_reads_case_and_expected_files");
        let case_path = temp_dir.write_file(
            "case.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-fixture"),
            case_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("fixture check succeeds on supported host");
        let expected =
            observed_result_from_json(include_str!("../../../tests/expected/return_42.json"))
                .and_then(|result| observed_result_to_json(&result))
                .expect("expected fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_executable_reads_manifest_and_expected_files() {
        let temp_dir = TestTempDir::new("check_executable_reads_manifest_and_expected_files");
        let manifest_path = temp_dir.write_file(
            "manifest.json",
            include_str!("../../../tests/executables/hello_world_executable_manifest.json"),
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/hello_world_executable_manifest.json"),
        );

        let output = run_cli(vec![
            String::from("check-executable"),
            manifest_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("executable manifest check succeeds on supported host");
        let expected = observed_result_from_json(include_str!(
            "../../../tests/expected/hello_world_executable_manifest.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("expected executable fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_mach_o_reads_binary_and_expected_files() {
        let temp_dir = TestTempDir::new("check_mach_o_reads_binary_and_expected_files");
        let binary_path = temp_dir.write_binary_file(
            "mach_o_return_42.bin",
            include_bytes!("../../../tests/binaries/mach_o_return_42.bin"),
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/mach_o_return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-mach-o"),
            binary_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("mach-o fixture check succeeds on supported host");
        let expected = observed_result_from_json(include_str!(
            "../../../tests/expected/mach_o_return_42.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("expected mach-o fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_mach_o_host_traps_reads_binary_plan_and_expected_files() {
        let temp_dir =
            TestTempDir::new("check_mach_o_host_traps_reads_binary_plan_and_expected_files");
        let binary_path = temp_dir.write_binary_file(
            "mach_o_hello_world_stdout.bin",
            include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
        );
        let host_traps_path =
            temp_dir.write_file("host-traps.json", MACH_O_HELLO_WORLD_STDOUT_HOST_TRAPS_JSON);
        let expected_path =
            temp_dir.write_file("expected.json", MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON);

        let output = run_cli(vec![
            String::from("check-mach-o-host-traps"),
            binary_path.to_string_lossy().into_owned(),
            host_traps_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("mach-o host trap fixture check succeeds on supported host");
        let expected = observed_result_from_json(MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON)
            .and_then(|result| observed_result_to_json(&result))
            .expect("expected mach-o host trap fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_mach_o_host_traps_reads_binary_metadata_and_expected_files() {
        let temp_dir =
            TestTempDir::new("check_mach_o_host_traps_reads_binary_metadata_and_expected_files");
        let binary_path = temp_dir.write_binary_file(
            "mach_o_hello_world_stdout.bin",
            include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
        );
        let expected_path =
            temp_dir.write_file("expected.json", MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON);

        let output = run_cli(vec![
            String::from("check-mach-o-host-traps"),
            binary_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("mach-o host trap metadata fixture check succeeds on supported host");
        let expected = observed_result_from_json(MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON)
            .and_then(|result| observed_result_to_json(&result))
            .expect("expected mach-o host trap fixture normalizes to output json");

        assert_eq!(output, expected);
    }

    #[test]
    fn check_mach_o_host_traps_does_not_derive_plan_from_expected_json() {
        let temp_dir =
            TestTempDir::new("check_mach_o_host_traps_does_not_derive_plan_from_expected_json");
        let binary_path = temp_dir.write_binary_file(
            "mach_o_hello_world_stdout.bin",
            include_bytes!("../../../tests/binaries/mach_o_hello_world_stdout.bin"),
        );
        let host_traps_path = temp_dir.write_file("host-traps.json", "{}");
        let expected_path =
            temp_dir.write_file("expected.json", MACH_O_HELLO_WORLD_STDOUT_EXPECTED_JSON);

        let error = run_cli(vec![
            String::from("check-mach-o-host-traps"),
            binary_path.to_string_lossy().into_owned(),
            host_traps_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect_err("missing explicit host trap plan fails comparison");

        assert!(matches!(error, CliError::Comparison(_)));
    }

    #[test]
    fn probe_binary_reads_file_and_reports_unsupported_mach_o() {
        let temp_dir = TestTempDir::new("probe_binary_reads_file_and_reports_unsupported_mach_o");
        let binary_path = temp_dir.write_binary_file(
            "fixture.bin",
            &[
                0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ],
        );

        let output = run_cli(vec![
            String::from("probe-binary"),
            binary_path.to_string_lossy().into_owned(),
        ])
        .expect("binary probe succeeds for recognized public binary format");

        assert_eq!(
            output,
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}}}}"
        );
    }

    #[test]
    fn check_binary_probe_compares_probe_report_with_expected_json() {
        let temp_dir =
            TestTempDir::new("check_binary_probe_compares_probe_report_with_expected_json");
        let binary_path = temp_dir.write_binary_file(
            "fixture.bin",
            &[
                0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ],
        );
        let expected_path = temp_dir.write_file(
            "expected.json",
            "{\n  \"format\": \"mach_o_64_little_endian\",\n  \"status\": \"recognized_but_unsupported\",\n  \"metadata\": {\n    \"mach_o\": {\n      \"file_type\": \"executable\",\n      \"load_commands\": {\n        \"count\": 0,\n        \"byte_size\": 0,\n        \"recognized_entry_points\": [],\n        \"recognized_segments\": [],\n        \"unsupported_commands\": []\n      },\n      \"executable_image_conversion\": {\n        \"status\": \"not_convertible\",\n        \"blocker\": \"missing_entry_point\"\n      }\n    }\n  }\n}\n",
        );

        let output = run_cli(vec![
            String::from("check-binary-probe"),
            binary_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("binary probe check succeeds");

        assert_eq!(
            output,
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}}}}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn build_x86_64_macho_fixture_writes_return_42_executable() {
        let temp_dir = TestTempDir::new("build_x86_64_macho_fixture_writes_return_42_executable");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let output_path = temp_dir.path.join("return_42_x86_64");

        let output = run_cli(vec![
            String::from("build-x86_64-macho-fixture"),
            case_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("return_42 builds as an x86_64 Mach-O fixture");

        assert!(output_path.exists());
        assert_eq!(
            output,
            format!(
                "{{\"artifact_kind\":\"mach_o_executable\",\"case_id\":\"return_42\",\"target_triple\":\"x86_64-apple-macos13\",\"toolchain\":\"clang\",\"output_path\":\"{}\"}}",
                output_path.display()
            )
        );

        let binary = fs::read(&output_path).expect("generated x86_64 Mach-O is readable");
        assert_eq!(&binary[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
        assert_eq!(&binary[4..8], &[0x07, 0x00, 0x00, 0x01]);
        let probe_output = run_cli(vec![
            String::from("probe-binary"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("generated x86_64 Mach-O probes as public Mach-O");
        assert!(probe_output.contains("\"format\":\"mach_o_64_little_endian\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn build_x86_64_gui_hello_world_fixture_writes_mach_o_executable() {
        let temp_dir =
            TestTempDir::new("build_x86_64_gui_hello_world_fixture_writes_mach_o_executable");
        let output_path = temp_dir.path.join("b8_gui_hello_world_x86_64");

        let output = run_cli(vec![
            String::from("build-x86_64-gui-hello-world-fixture"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("B8 GUI Hello World builds as an x86_64 Mach-O executable");

        assert!(output_path.exists());
        assert_eq!(
            output,
            format!(
                "{{\"artifact_kind\":\"gui_hello_world_mach_o_executable\",\"case_id\":\"b8_gui_hello_world\",\"target_triple\":\"x86_64-apple-macos13\",\"toolchain\":\"clang\",\"output_path\":\"{}\"}}",
                output_path.display()
            )
        );

        let binary = fs::read(&output_path).expect("generated GUI fixture is readable");
        assert_eq!(&binary[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
        assert_eq!(&binary[4..8], &[0x07, 0x00, 0x00, 0x01]);
        let probe_output = run_cli(vec![
            String::from("probe-binary"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("generated GUI fixture probes as public Mach-O");
        assert!(probe_output.contains("\"format\":\"mach_o_64_little_endian\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn build_x86_64_gui_hello_world_visible_fixture_writes_mach_o_executable() {
        let temp_dir = TestTempDir::new(
            "build_x86_64_gui_hello_world_visible_fixture_writes_mach_o_executable",
        );
        let output_path = temp_dir.path.join("b8_gui_hello_world_visible_x86_64");

        let output = run_cli(vec![
            String::from("build-x86_64-gui-hello-world-visible-fixture"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("B8 visible GUI Hello World builds as an x86_64 Mach-O executable");

        assert!(output_path.exists());
        assert_eq!(
            output,
            format!(
                "{{\"artifact_kind\":\"gui_hello_world_manual_visible_mach_o_executable\",\"case_id\":\"b8_gui_hello_world\",\"target_triple\":\"x86_64-apple-macos13\",\"toolchain\":\"clang\",\"output_path\":\"{}\"}}",
                output_path.display()
            )
        );

        let binary = fs::read(&output_path).expect("generated visible GUI fixture is readable");
        assert_eq!(&binary[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
        assert_eq!(&binary[4..8], &[0x07, 0x00, 0x00, 0x01]);
        let probe_output = run_cli(vec![
            String::from("probe-binary"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("generated visible GUI fixture probes as public Mach-O");
        assert!(probe_output.contains("\"format\":\"mach_o_64_little_endian\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn build_x86_64_oracle_runner_writes_return_42_runner_executable() {
        let temp_dir =
            TestTempDir::new("build_x86_64_oracle_runner_writes_return_42_runner_executable");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let output_path = temp_dir.path.join("return_42_oracle");

        let output = run_cli(vec![
            String::from("build-x86_64-oracle-runner"),
            case_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("return_42 builds as an x86_64 oracle runner");

        assert!(output_path.exists());
        assert_eq!(
            output,
            format!(
                "{{\"artifact_kind\":\"oracle_runner_executable\",\"case_id\":\"return_42\",\"target_triple\":\"x86_64-apple-macos13\",\"toolchain\":\"clang\",\"output_path\":\"{}\"}}",
                output_path.display()
            )
        );

        let binary = fs::read(&output_path).expect("generated x86_64 oracle runner is readable");
        assert_eq!(&binary[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
        assert_eq!(&binary[4..8], &[0x07, 0x00, 0x00, 0x01]);
        let probe_output = run_cli(vec![
            String::from("probe-binary"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect("generated x86_64 oracle runner probes as public Mach-O");
        assert!(probe_output.contains("\"format\":\"mach_o_64_little_endian\""));
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn generate_x86_64_expected_writes_return_42_expected_json() {
        let temp_dir = TestTempDir::new("generate_x86_64_expected_writes_return_42_expected_json");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let expected_path = temp_dir.path.join("expected").join("return_42.json");

        let output = run_cli(vec![
            String::from("generate-x86_64-expected"),
            case_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect("return_42 expected JSON is generated under Rosetta");
        let expected =
            observed_result_from_json(include_str!("../../../tests/expected/return_42.json"))
                .and_then(|result| observed_result_to_json(&result))
                .expect("return_42 expected fixture normalizes to output json");

        assert_eq!(output, expected);
        assert_eq!(read_file(&expected_path), expected);
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn generate_x86_64_gui_hello_world_expected_writes_expected_and_launch_metadata() {
        let temp_dir = TestTempDir::new(
            "generate_x86_64_gui_hello_world_expected_writes_expected_and_launch_metadata",
        );
        let expected_path = temp_dir
            .path
            .join("expected")
            .join("b8_gui_hello_world.json");
        let launch_metadata_path = temp_dir
            .path
            .join("expected")
            .join("b8_gui_hello_world.launch.metadata.json");

        let output = run_cli(vec![
            String::from("generate-x86_64-gui-hello-world-expected"),
            expected_path.to_string_lossy().into_owned(),
            launch_metadata_path.to_string_lossy().into_owned(),
        ])
        .expect("B8 GUI Hello World expected JSON is generated under Rosetta");

        assert_eq!(
            output,
            format!(
                "{{\"expected\":\"{}\",\"launch_metadata\":\"{}\"}}",
                expected_path.display(),
                launch_metadata_path.display()
            )
        );
        let expected = observed_result_from_json(include_str!(
            "../../../tests/expected/b8_gui_hello_world.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("B8 GUI expected fixture normalizes to output json");
        assert_eq!(read_file(&expected_path), expected);
        assert_eq!(
            read_file(&launch_metadata_path),
            include_str!("../../../tests/expected/b8_gui_hello_world.launch.metadata.json")
                .trim_end_matches('\n')
        );
    }

    #[cfg(all(unix, target_arch = "aarch64"))]
    #[test]
    fn generate_arm64_actual_writes_return_42_actual_json() {
        let temp_dir = TestTempDir::new("generate_arm64_actual_writes_return_42_actual_json");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let actual_path = temp_dir.path.join("actual").join("return_42.json");

        let output = run_cli(vec![
            String::from("generate-arm64-actual"),
            case_path.to_string_lossy().into_owned(),
            actual_path.to_string_lossy().into_owned(),
        ])
        .expect("return_42 actual JSON is generated by the ARM64 native runner");
        let expected =
            observed_result_from_json(include_str!("../../../tests/expected/return_42.json"))
                .and_then(|result| observed_result_to_json(&result))
                .expect("return_42 expected fixture normalizes to output json");

        assert_eq!(output, expected);
        assert_eq!(read_file(&actual_path), expected);
    }

    #[test]
    fn generate_arm64_gui_hello_world_actual_writes_actual_and_launch_report() {
        let temp_dir = TestTempDir::new(
            "generate_arm64_gui_hello_world_actual_writes_actual_and_launch_report",
        );
        let binary_path = temp_dir.write_binary_file(
            "b8_gui_hello_world",
            include_bytes!("../../../tests/binaries/mach_o_execute_header.bin"),
        );
        let actual_path = temp_dir.path.join("actual").join("b8_gui_hello_world.json");
        let launch_report_path = temp_dir
            .path
            .join("actual")
            .join("b8_gui_hello_world.launch-report.json");

        let output = run_cli(vec![
            String::from("generate-arm64-gui-hello-world-actual"),
            binary_path.to_string_lossy().into_owned(),
            actual_path.to_string_lossy().into_owned(),
            launch_report_path.to_string_lossy().into_owned(),
        ])
        .expect("B8 GUI Hello World actual matched report is generated");

        assert_eq!(
            output,
            format!(
                "{{\"actual\":\"{}\",\"launch_report\":\"{}\"}}",
                actual_path.display(),
                launch_report_path.display()
            )
        );
        let expected_actual = observed_result_from_json(include_str!(
            "../../../tests/expected/b8_gui_hello_world.bara.actual.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("B8 GUI actual matched fixture normalizes to output json");
        assert_eq!(read_file(&actual_path), expected_actual);
        assert_eq!(
            read_file(&launch_report_path),
            include_str!("../../../tests/expected/b8_gui_hello_world.bara.launch-report.json")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn generate_arm64_gui_hello_world_feedback_writes_matched_report() {
        let temp_dir =
            TestTempDir::new("generate_arm64_gui_hello_world_feedback_writes_matched_report");
        let binary_path = temp_dir.write_binary_file(
            "b8_gui_hello_world",
            include_bytes!("../../../tests/binaries/mach_o_execute_header.bin"),
        );
        let expected_path = temp_dir.write_file(
            "b8_gui_hello_world.expected.json",
            include_str!("../../../tests/expected/b8_gui_hello_world.json"),
        );
        let actual_path = temp_dir.path.join("actual").join("b8_gui_hello_world.json");
        let launch_report_path = temp_dir
            .path
            .join("actual")
            .join("b8_gui_hello_world.launch-report.json");
        let feedback_report_path = temp_dir
            .path
            .join("actual")
            .join("b8_gui_hello_world.feedback-report.json");

        let output = run_cli(vec![
            String::from("generate-arm64-gui-hello-world-feedback"),
            binary_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
            actual_path.to_string_lossy().into_owned(),
            launch_report_path.to_string_lossy().into_owned(),
            feedback_report_path.to_string_lossy().into_owned(),
        ])
        .expect("B8 GUI Hello World feedback report is generated");

        assert_eq!(
            output,
            format!(
                "{{\"actual\":\"{}\",\"launch_report\":\"{}\",\"feedback_report\":\"{}\"}}",
                actual_path.display(),
                launch_report_path.display(),
                feedback_report_path.display()
            )
        );
        let expected_actual = observed_result_from_json(include_str!(
            "../../../tests/expected/b8_gui_hello_world.bara.actual.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("B8 GUI actual matched fixture normalizes to output json");
        assert_eq!(read_file(&actual_path), expected_actual);
        assert_eq!(
            read_file(&launch_report_path),
            include_str!("../../../tests/expected/b8_gui_hello_world.bara.launch-report.json")
                .trim_end_matches('\n')
        );
        assert_eq!(
            read_file(&feedback_report_path),
            include_str!("../../../tests/expected/b8_gui_hello_world.bara.feedback-report.json")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn generate_arm64_gui_hello_world_translated_actual_writes_runtime_path_report() {
        let temp_dir =
            TestTempDir::new("generate_arm64_gui_hello_world_translated_actual_writes_report");
        let binary_path = temp_dir.write_binary_file(
            "b8_gui_hello_world",
            include_bytes!("../../../tests/binaries/mach_o_execute_header.bin"),
        );
        let actual_path = temp_dir
            .path
            .join("actual")
            .join("b8_gui_hello_world.translated.json");
        let launch_report_path = temp_dir
            .path
            .join("actual")
            .join("b8_gui_hello_world.translated.launch-report.json");

        let output = run_cli(vec![
            String::from("generate-arm64-gui-hello-world-translated-actual"),
            binary_path.to_string_lossy().into_owned(),
            actual_path.to_string_lossy().into_owned(),
            launch_report_path.to_string_lossy().into_owned(),
        ])
        .expect("B8 translated GUI actual report is generated");

        assert_eq!(
            output,
            format!(
                "{{\"actual\":\"{}\",\"launch_report\":\"{}\"}}",
                actual_path.display(),
                launch_report_path.display()
            )
        );
        let expected_actual = observed_result_from_json(include_str!(
            "../../../tests/expected/b8_gui_hello_world.bara.actual.json"
        ))
        .and_then(|result| observed_result_to_json(&result))
        .expect("B8 GUI actual fixture normalizes to output json");
        assert_eq!(read_file(&actual_path), expected_actual);

        let report = read_file(&launch_report_path);
        assert!(report.contains("\"schema\":\"b8_gui_hello_world_translated_launch_report_v0\""));
        assert!(report.contains("\"source_bytes\":\"0f0b4238473131c0c3\""));
        assert!(report.contains("\"kind\":\"appkit_gui_hello_world\",\"requested\":true"));
        assert!(report.contains("\"invoked_by\":\"translated_host_trap_request\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary() {
        let temp_dir =
            TestTempDir::new("generate_b8_debug_bundle_reports_call_r14_as_indirect_call_boundary");
        let binary_path = temp_dir.path.join("b8_gui_hello_world_x86_64");
        let output_root = temp_dir.path.join("b8-debug");
        let bundle_dir = output_root.join("b8_gui_hello_world");

        run_cli(vec![
            String::from("build-x86_64-gui-hello-world-fixture"),
            binary_path.to_string_lossy().into_owned(),
        ])
        .expect("B8 GUI Hello World fixture is generated");

        let output = run_cli(vec![
            String::from("generate-b8-debug-bundle"),
            binary_path.to_string_lossy().into_owned(),
            output_root.to_string_lossy().into_owned(),
        ])
        .expect("B8 debug bundle is generated");

        assert_eq!(
            output,
            format!(
                "{{\"bundle_dir\":\"{}\",\"input_probe\":\"{}\",\"entry_bytes_bin\":\"{}\",\"entry_bytes_json\":\"{}\",\"decode_report\":\"{}\",\"lift_ir\":\"{}\",\"emit_report\":\"{}\",\"pcmap\":\"{}\",\"fixups\":\"{}\",\"helpers\":\"{}\",\"loader_plan\":\"{}\",\"runtime_attempt\":\"{}\",\"launch_report\":\"{}\",\"blocker\":\"{}\",\"repro\":\"{}\"}}",
                bundle_dir.display(),
                bundle_dir.join("input.probe.json").display(),
                bundle_dir.join("entry.bytes.bin").display(),
                bundle_dir.join("entry.bytes.json").display(),
                bundle_dir.join("decode.report.json").display(),
                bundle_dir.join("lift.ir.json").display(),
                bundle_dir.join("emit.report.json").display(),
                bundle_dir.join("pcmap.json").display(),
                bundle_dir.join("fixups.json").display(),
                bundle_dir.join("helpers.json").display(),
                bundle_dir.join("loader.plan.json").display(),
                bundle_dir.join("runtime-attempt.json").display(),
                bundle_dir.join("launch.report.json").display(),
                bundle_dir.join("blocker.json").display(),
                bundle_dir.join("repro.sh").display(),
            )
        );
        let entry_bytes =
            fs::read(bundle_dir.join("entry.bytes.bin")).expect("entry bytes are readable");
        assert!(!entry_bytes.is_empty());
        assert_ne!(
            entry_bytes,
            vec![0x0f, 0x0b, b'B', b'8', b'G', b'1', 0x31, 0xc0, 0xc3]
        );
        assert!(read_file(&bundle_dir.join("input.probe.json"))
            .contains("\"format\":\"mach_o_64_little_endian\""));
        assert!(read_file(&bundle_dir.join("entry.bytes.json"))
            .contains("\"source\":\"public_lc_main_entryoff\""));
        let decode_report = read_file(&bundle_dir.join("decode.report.json"));
        assert!(decode_report.contains("\"schema\":\"b8_debug_decode_report_v0\""));
        assert!(decode_report.contains("\"kind\":\"push_rbp\""));
        assert!(decode_report.contains("\"kind\":\"mov_rbp_rsp\""));
        assert!(decode_report.contains("\"kind\":\"push_r15\""));
        assert!(decode_report.contains("\"kind\":\"push_r14\""));
        assert!(decode_report.contains("\"kind\":\"push_rbx\""));
        assert!(decode_report.contains("\"kind\":\"mov_rbx_rax\""));
        assert!(decode_report.contains("\"kind\":\"mov_rax_qword_ptr_rip_relative\""));
        assert!(decode_report.contains("\"kind\":\"mov_rdx_qword_ptr_rax\""));
        assert!(decode_report.contains("\"kind\":\"lea_rdi_rip_relative\""));
        assert!(decode_report.contains("\"kind\":\"lea_rsi_rip_relative\""));
        assert!(decode_report.contains("\"kind\":\"mov_rdi_qword_ptr_rip_relative\""));
        assert!(decode_report.contains("\"kind\":\"mov_rsi_qword_ptr_rip_relative\""));
        assert!(decode_report.contains("\"kind\":\"mov_r14_qword_ptr_rip_relative\""));
        assert!(decode_report.contains("\"kind\":\"call_r14\""));
        assert!(decode_report.contains("\"kind\":\"call_rel32\""));
        assert!(decode_report.contains("\"width\":\"bits64\""));
        assert!(!decode_report.contains("DecodeUnsupportedOpcode { opcode: 65"));
        assert!(read_file(&bundle_dir.join("lift.ir.json")).contains("\"status\":"));
        assert!(read_file(&bundle_dir.join("emit.report.json")).contains("\"status\":"));
        assert!(read_file(&bundle_dir.join("pcmap.json")).contains("\"status\":"));
        assert!(read_file(&bundle_dir.join("fixups.json")).contains("\"status\":"));
        assert!(read_file(&bundle_dir.join("helpers.json")).contains("\"status\":"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"entry_source_for_this_bundle\":\"public_lc_main_entryoff\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"segment_source\":\"lc_segment64_file_range\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"address_space\":\"mach_o_virtual_address\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"entry_pc\":4294972928"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_action\":\"resolve_public_rebase_bind_imports\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"import_boundary\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"status\":\"blocked\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"target_register\":\"r14\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"target_pointer_load\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"address\":4294979672"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"command\":\"dyld_chained_fixups\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"dataoff\":24576"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"datasize\":584"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"chained_fixups\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"mach_o_chained_fixups_target_report_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"status\":\"resolved_import\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"imports_format\":{\"value\":1,\"kind\":\"dyld_chained_import\"}"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"pointer_format\":{\"value\":6,\"kind\":\"ptr64_offset\"}"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"symbol_name\":\"_objc_msgSend\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"dylib_path\":\"/usr/lib/libobjc.A.dylib\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"helper_boundary_request\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"reason\":\"return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented\""
        ));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"kind\":\"import_helper_call\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"public_dyld_chained_fixups_import\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"source_isa\":\"x86_64\"")
        );
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"call_site\":4294972996")
        );
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_to\":4294972999")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"argument_model\":\"x86_64_call_arguments\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"return_model\":\"x86_64_rax_return_value\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_import_helper_marshaling_contract_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"calling_convention\":\"x86_64_macos_system_v\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"argument_sources\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"position\":0"));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"role\":\"objc_receiver\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"position\":1"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"rdi\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"role\":\"objc_selector\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"rsi\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_destination\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"rax\""));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"objc_receiver_materialization_unimplemented\""));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"objc_selector_materialization_unimplemented\""));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"helper_return_value_materialization_unimplemented\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_objc_helper_return_writeback_boundary_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"objc_helper_return_value\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"destination\":\"x86_64_rax\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"writeback_plan\":\"write_helper_return_to_x86_64_rax\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"ordering\":\"after_helper_call_returns\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"objc_helper_execution_unimplemented\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"helper_execution_request\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_objc_helper_execution_request_v0\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"kind\":\"objc_msg_send\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"source_import\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"receiver_identity\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"selector_vm_address\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"required_capability\":\"objc_runtime_message_send_helper\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"bridge_contract\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_objc_runtime_helper_bridge_contract_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"input_contract\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"output_contract\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"error_contract\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"helper_output\":\"objc_helper_return_value\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"error_classification\":null"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"host_execution\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_objc_runtime_helper_host_execution_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"status\":\"executed\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"api_boundary\":\"public_objc_runtime_appkit\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"fixture_scope\":\"self_authored_b8_gui_fixture\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"selector_identity\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"name\":\"sharedApplication\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"function\":\"_objc_msgSend\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"receiver\":\"ns_application_class_object\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"selector_name\":\"sharedApplication\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"representation\":\"host_pointer_u64\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_value\":"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_writeback\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"written_value\":"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_blocker\":\"objc_helper_return_continuation_unimplemented\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_action\":\"continue_after_objc_helper_return\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_continuation\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_objc_helper_return_continuation_boundary_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"kind\":\"register_indirect_call_return\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_source_pc\":4294972999"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register_state\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"blocker\":\"return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented\""
        ));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"continuation_block\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_return_to_continuation_decode_boundary_v0\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"source_pc\":4294972999")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"byte_source\":\"mach_o_code_segment_bytes\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"input_register_state\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"decode_report\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"entry\":4294972999"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"next_instruction\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_action\":\"inspect_return_to_continuation_blocker\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"start\":4294972999"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"end\":4294973006"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"kind\":\"mov_r15_qword_ptr_rip_relative\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"displacement\":\"X86Imm32 { value: 6578 }\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"address\":4294979584"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"materialized_register_states\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"r15\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"rip_relative_qword_load\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"value\":9227875636482146304"));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"symbol_name\":\"_NSApp\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"end\":4294973009"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"kind\":\"mov_rdi_qword_ptr_r15\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"blocked_register_materializations\":[]"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"rdi\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"imported_global_pointee_load\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"base_register\":\"r15\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"base_value\":9227875636482146304"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"value_source\":\"objc_shared_application_helper_return_value\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"end\":4294973018"));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"kind\":\"xor_edx_edx\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"rdx\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"xor_edx_edx_zero\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"value\":0"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"rsi\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"address\":4294988008"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"kind\":\"call_r14\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_to\":4294973021")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"continuation_call_boundary\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_return_to_continuation_call_boundary_v0\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"objc_helper_boundary\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_return_to_continuation_objc_helper_boundary_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"helper_request\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"bridge_contract\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"available_or_blocked_state\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"receiver_state\":\"available\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"selector_state\":\"available\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"argument_state\":\"available\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"execution_state\":\"executed\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"selector_name\":\"setActivationPolicy:\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"argument_value\":0"));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"call_site\":4294973018")
        );
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"target_register\":\"r14\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"preserved_import_helper_call_target\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"preservation_model\":\"x86_64_macos_system_v_callee_saved_register\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"arguments\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"role\":\"objc_argument\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"state\":\"available\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"selector_identity\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"name\":\"setActivationPolicy:\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"resolved_vm_address\":4294975544"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"host_execution\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_return_to_continuation_objc_helper_host_execution_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"status\":\"executed\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"effect\":\"set_activation_policy\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"representation\":\"bool_as_u64\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_value\":"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_source_pc\":4294973021"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"next_continuation\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"source_pc\":4294973021")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"kind\":\"mov_rdi_qword_ptr_rip_relative\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"kind\":\"mov_rdx_rax\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"register_copy_from_rax\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"source_register\":\"rax\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"source_call_return\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_return_to_continuation_call_rel32_helper_boundary_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_return_to_continuation_mach_o_stub_symbol_resolution_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"section_name\":\"__stubs\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"stub_address\":4294973108")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"stub_byte_size\":6"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"stub_index\":4"));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"symbol_table_index\":46")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"symbol_name\":\"_objc_alloc_init\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"schema\":\"b8_return_to_continuation_call_rel32_helper_execution_request_v0\""
        ));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"kind\":\"objc_alloc_init\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"required_capability\":\"objc_alloc_init_helper\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"class_argument\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"role\":\"objc_class\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"register\":\"rdi\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"address\":4294988128"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"fixup_resolution\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"resolved_rebase\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_return_to_continuation_objc_alloc_init_class_bridge_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"schema\":\"b8_return_to_continuation_objc_alloc_init_class_identity_v0\""
        ));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"public_mach_o_symtab_nlist64\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"class_symbol_name\":\"_OBJC_CLASS_$_BaraGuiHelloWorldDelegate\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"class_name\":\"BaraGuiHelloWorldDelegate\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"symbol_vm_address\":4294988184"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"bridge_state\":\"fixture_delegate_bridge_unimplemented\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"schema\":\"b8_return_to_continuation_call_rel32_return_value_dataflow_v0\""
        ));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"consumer_register\":\"rdx\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"consumer_source_register\":\"rax\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"call_site\":4294973028")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"target\":4294973108"));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"return_register\":\"rax\"")
        );
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"name\":\"setDelegate:\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"blocker\":\"return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented\""
        ));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"unsupported_instruction\":null"));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"return_to_continuation_unsupported_instruction\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"next_action\":\"define_return_to_continuation_objc_alloc_init_fixture_delegate_bridge\""
        ));
        assert!(!read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"return_to_continuation_call_rel32_return_value_materialization_unimplemented\""
        ));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"return_to_continuation_objc_helper_execution_unimplemented\""));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"return_to_continuation_import_global_load_unimplemented\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"error\":null"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"materialization_boundary\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"schema\":\"b8_objc_message_materialization_boundary_v0\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"receiver\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"selector\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"source_definition\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"kind\":\"rip_relative_qword_load\""));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"target_register\":\"rdi\"")
        );
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"target_register\":\"rsi\"")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"mapped_value\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"program_image_metadata\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"address\":4294988120"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"value\":9227875636482146321"));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"address\":4294988072"));
        assert!(
            read_file(&bundle_dir.join("loader.plan.json")).contains("\"value\":4503599627378848")
        );
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"fixup_resolution\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"source\":\"public_dyld_chained_fixups\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"symbol_name\":\"_OBJC_CLASS_$_NSApplication\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains(
            "\"dylib_path\":\"/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit\""
        ));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"resolved_vm_address\":4294975648"));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"receiver_mapped_value_fixup_resolution_unimplemented\""));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"selector_mapped_value_fixup_resolution_unimplemented\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json")).contains("\"blocker\":null"));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_action\":\"continue_after_objc_helper_return\""));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"x86_64_argument_marshaling_unimplemented\""));
        assert!(!read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"helper_return_marshaling_unimplemented\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"resolution\":\"resolved_public_dyld_chained_fixups_import\""));
        assert!(read_file(&bundle_dir.join("loader.plan.json"))
            .contains("\"next_entry_source\":\"first_unsupported_boundary\""));
        assert!(read_file(&bundle_dir.join("runtime-attempt.json"))
            .contains("\"run_scope\":\"real_lc_main_entry_first_block\""));
        let launch_report = read_file(&bundle_dir.join("launch.report.json"));
        assert!(launch_report.contains("\"schema\":\"b8_debug_real_entry_launch_report_v0\""));
        assert!(launch_report.contains("\"entry_source\":\"public_lc_main_entryoff\""));
        assert!(launch_report.contains("\"processed_source_pc_range\":{\"start\":"));
        assert!(launch_report.contains("\"b8_g1_host_trap_path\":\"not_used\""));
        assert!(launch_report.contains("\"helper_boundary_request\""));
        assert!(launch_report.contains(
            "\"reason\":\"return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented\""
        ));
        assert!(launch_report.contains("\"symbol_name\":\"_objc_msgSend\""));
        assert!(launch_report.contains("\"call_site\":4294972996"));
        assert!(launch_report.contains("\"return_to\":4294972999"));
        assert!(launch_report.contains("\"schema\":\"b8_import_helper_marshaling_contract_v0\""));
        assert!(launch_report.contains("\"calling_convention\":\"x86_64_macos_system_v\""));
        assert!(launch_report.contains("\"role\":\"objc_receiver\""));
        assert!(launch_report.contains("\"role\":\"objc_selector\""));
        assert!(launch_report.contains("\"return_destination\""));
        assert!(!launch_report.contains("\"helper_return_value_materialization_unimplemented\""));
        assert!(
            launch_report.contains("\"schema\":\"b8_objc_helper_return_writeback_boundary_v0\"")
        );
        assert!(launch_report.contains("\"source\":\"objc_helper_return_value\""));
        assert!(launch_report.contains("\"destination\":\"x86_64_rax\""));
        assert!(launch_report.contains("\"writeback_plan\":\"write_helper_return_to_x86_64_rax\""));
        assert!(launch_report.contains("\"ordering\":\"after_helper_call_returns\""));
        assert!(launch_report.contains("\"objc_helper_execution_unimplemented\""));
        assert!(launch_report.contains("\"helper_execution_request\""));
        assert!(launch_report.contains("\"schema\":\"b8_objc_helper_execution_request_v0\""));
        assert!(launch_report.contains("\"kind\":\"objc_msg_send\""));
        assert!(launch_report.contains("\"source_import\""));
        assert!(launch_report.contains("\"receiver_identity\""));
        assert!(launch_report.contains("\"selector_vm_address\""));
        assert!(
            launch_report.contains("\"required_capability\":\"objc_runtime_message_send_helper\"")
        );
        assert!(launch_report.contains("\"bridge_contract\""));
        assert!(launch_report.contains("\"schema\":\"b8_objc_runtime_helper_bridge_contract_v0\""));
        assert!(launch_report.contains("\"input_contract\""));
        assert!(launch_report.contains("\"output_contract\""));
        assert!(launch_report.contains("\"error_contract\""));
        assert!(launch_report.contains("\"helper_output\":\"objc_helper_return_value\""));
        assert!(launch_report.contains("\"error_classification\":null"));
        assert!(launch_report.contains("\"host_execution\""));
        assert!(launch_report.contains("\"schema\":\"b8_objc_runtime_helper_host_execution_v0\""));
        assert!(launch_report.contains("\"status\":\"executed\""));
        assert!(launch_report.contains("\"api_boundary\":\"public_objc_runtime_appkit\""));
        assert!(launch_report.contains("\"fixture_scope\":\"self_authored_b8_gui_fixture\""));
        assert!(launch_report.contains("\"selector_identity\""));
        assert!(launch_report.contains("\"name\":\"sharedApplication\""));
        assert!(launch_report.contains("\"function\":\"_objc_msgSend\""));
        assert!(launch_report.contains("\"receiver\":\"ns_application_class_object\""));
        assert!(launch_report.contains("\"selector_name\":\"sharedApplication\""));
        assert!(launch_report.contains("\"representation\":\"host_pointer_u64\""));
        assert!(launch_report.contains("\"return_value\":"));
        assert!(launch_report.contains("\"return_writeback\""));
        assert!(launch_report.contains("\"written_value\":"));
        assert!(launch_report
            .contains("\"next_blocker\":\"objc_helper_return_continuation_unimplemented\""));
        assert!(launch_report.contains("\"next_action\":\"continue_after_objc_helper_return\""));
        assert!(launch_report.contains("\"return_continuation\""));
        assert!(
            launch_report.contains("\"schema\":\"b8_objc_helper_return_continuation_boundary_v0\"")
        );
        assert!(launch_report.contains("\"kind\":\"register_indirect_call_return\""));
        assert!(launch_report.contains("\"next_source_pc\":4294972999"));
        assert!(launch_report.contains("\"register_state\""));
        assert!(launch_report.contains(
            "\"blocker\":\"return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented\""
        ));
        assert!(launch_report.contains("\"continuation_block\""));
        assert!(
            launch_report.contains("\"schema\":\"b8_return_to_continuation_decode_boundary_v0\"")
        );
        assert!(launch_report.contains("\"source_pc\":4294972999"));
        assert!(launch_report.contains("\"byte_source\":\"mach_o_code_segment_bytes\""));
        assert!(launch_report.contains("\"input_register_state\""));
        assert!(launch_report.contains("\"decode_report\""));
        assert!(launch_report.contains("\"entry\":4294972999"));
        assert!(launch_report.contains("\"next_instruction\""));
        assert!(
            launch_report.contains("\"next_action\":\"inspect_return_to_continuation_blocker\"")
        );
        assert!(launch_report.contains("\"start\":4294972999"));
        assert!(launch_report.contains("\"end\":4294973006"));
        assert!(launch_report.contains("\"kind\":\"mov_r15_qword_ptr_rip_relative\""));
        assert!(launch_report.contains("\"displacement\":\"X86Imm32 { value: 6578 }\""));
        assert!(launch_report.contains("\"address\":4294979584"));
        assert!(launch_report.contains("\"materialized_register_states\""));
        assert!(launch_report.contains("\"register\":\"r15\""));
        assert!(launch_report.contains("\"source\":\"rip_relative_qword_load\""));
        assert!(launch_report.contains("\"value\":9227875636482146304"));
        assert!(launch_report.contains("\"symbol_name\":\"_NSApp\""));
        assert!(launch_report.contains("\"end\":4294973009"));
        assert!(launch_report.contains("\"kind\":\"mov_rdi_qword_ptr_r15\""));
        assert!(launch_report.contains("\"blocked_register_materializations\":[]"));
        assert!(launch_report.contains("\"register\":\"rdi\""));
        assert!(launch_report.contains("\"source\":\"imported_global_pointee_load\""));
        assert!(launch_report.contains("\"base_register\":\"r15\""));
        assert!(launch_report.contains("\"base_value\":9227875636482146304"));
        assert!(launch_report
            .contains("\"value_source\":\"objc_shared_application_helper_return_value\""));
        assert!(launch_report.contains("\"end\":4294973018"));
        assert!(launch_report.contains("\"kind\":\"xor_edx_edx\""));
        assert!(launch_report.contains("\"register\":\"rdx\""));
        assert!(launch_report.contains("\"source\":\"xor_edx_edx_zero\""));
        assert!(launch_report.contains("\"value\":0"));
        assert!(launch_report.contains("\"register\":\"rsi\""));
        assert!(launch_report.contains("\"address\":4294988008"));
        assert!(launch_report.contains("\"kind\":\"call_r14\""));
        assert!(launch_report.contains("\"return_to\":4294973021"));
        assert!(launch_report.contains("\"continuation_call_boundary\""));
        assert!(launch_report.contains("\"schema\":\"b8_return_to_continuation_call_boundary_v0\""));
        assert!(launch_report.contains("\"objc_helper_boundary\""));
        assert!(launch_report
            .contains("\"schema\":\"b8_return_to_continuation_objc_helper_boundary_v0\""));
        assert!(launch_report.contains("\"helper_request\""));
        assert!(launch_report.contains("\"bridge_contract\""));
        assert!(launch_report.contains("\"available_or_blocked_state\""));
        assert!(launch_report.contains("\"receiver_state\":\"available\""));
        assert!(launch_report.contains("\"selector_state\":\"available\""));
        assert!(launch_report.contains("\"argument_state\":\"available\""));
        assert!(launch_report.contains("\"execution_state\":\"executed\""));
        assert!(launch_report.contains("\"selector_name\":\"setActivationPolicy:\""));
        assert!(launch_report.contains("\"argument_value\":0"));
        assert!(launch_report.contains("\"call_site\":4294973018"));
        assert!(launch_report.contains("\"target_register\":\"r14\""));
        assert!(launch_report.contains("\"source\":\"preserved_import_helper_call_target\""));
        assert!(launch_report
            .contains("\"preservation_model\":\"x86_64_macos_system_v_callee_saved_register\""));
        assert!(launch_report.contains("\"arguments\""));
        assert!(launch_report.contains("\"role\":\"objc_argument\""));
        assert!(launch_report.contains("\"state\":\"available\""));
        assert!(launch_report.contains("\"selector_identity\""));
        assert!(launch_report.contains("\"name\":\"setActivationPolicy:\""));
        assert!(launch_report.contains("\"resolved_vm_address\":4294975544"));
        assert!(launch_report.contains("\"host_execution\""));
        assert!(launch_report
            .contains("\"schema\":\"b8_return_to_continuation_objc_helper_host_execution_v0\""));
        assert!(launch_report.contains("\"status\":\"executed\""));
        assert!(launch_report.contains("\"effect\":\"set_activation_policy\""));
        assert!(launch_report.contains("\"representation\":\"bool_as_u64\""));
        assert!(launch_report.contains("\"return_value\":"));
        assert!(launch_report.contains("\"next_source_pc\":4294973021"));
        assert!(launch_report.contains("\"next_continuation\""));
        assert!(launch_report.contains("\"source_pc\":4294973021"));
        assert!(launch_report.contains("\"kind\":\"mov_rdi_qword_ptr_rip_relative\""));
        assert!(launch_report.contains("\"kind\":\"mov_rdx_rax\""));
        assert!(launch_report.contains("\"source\":\"register_copy_from_rax\""));
        assert!(launch_report.contains("\"source_register\":\"rax\""));
        assert!(launch_report.contains("\"source_call_return\""));
        assert!(launch_report
            .contains("\"schema\":\"b8_return_to_continuation_call_rel32_helper_boundary_v0\""));
        assert!(launch_report
            .contains("\"schema\":\"b8_return_to_continuation_mach_o_stub_symbol_resolution_v0\""));
        assert!(launch_report.contains("\"section_name\":\"__stubs\""));
        assert!(launch_report.contains("\"stub_address\":4294973108"));
        assert!(launch_report.contains("\"stub_byte_size\":6"));
        assert!(launch_report.contains("\"stub_index\":4"));
        assert!(launch_report.contains("\"symbol_table_index\":46"));
        assert!(launch_report.contains("\"symbol_name\":\"_objc_alloc_init\""));
        assert!(launch_report.contains(
            "\"schema\":\"b8_return_to_continuation_call_rel32_helper_execution_request_v0\""
        ));
        assert!(launch_report.contains("\"kind\":\"objc_alloc_init\""));
        assert!(launch_report.contains("\"required_capability\":\"objc_alloc_init_helper\""));
        assert!(launch_report.contains("\"class_argument\""));
        assert!(launch_report.contains("\"role\":\"objc_class\""));
        assert!(launch_report.contains("\"register\":\"rdi\""));
        assert!(launch_report.contains("\"address\":4294988128"));
        assert!(launch_report.contains("\"fixup_resolution\""));
        assert!(launch_report.contains("\"resolved_rebase\""));
        assert!(launch_report
            .contains("\"schema\":\"b8_return_to_continuation_objc_alloc_init_class_bridge_v0\""));
        assert!(launch_report.contains(
            "\"schema\":\"b8_return_to_continuation_objc_alloc_init_class_identity_v0\""
        ));
        assert!(launch_report.contains("\"source\":\"public_mach_o_symtab_nlist64\""));
        assert!(launch_report
            .contains("\"class_symbol_name\":\"_OBJC_CLASS_$_BaraGuiHelloWorldDelegate\""));
        assert!(launch_report.contains("\"class_name\":\"BaraGuiHelloWorldDelegate\""));
        assert!(launch_report.contains("\"symbol_vm_address\":4294988184"));
        assert!(
            launch_report.contains("\"bridge_state\":\"fixture_delegate_bridge_unimplemented\"")
        );
        assert!(launch_report.contains(
            "\"schema\":\"b8_return_to_continuation_call_rel32_return_value_dataflow_v0\""
        ));
        assert!(launch_report.contains("\"consumer_register\":\"rdx\""));
        assert!(launch_report.contains("\"consumer_source_register\":\"rax\""));
        assert!(launch_report.contains("\"call_site\":4294973028"));
        assert!(launch_report.contains("\"target\":4294973108"));
        assert!(launch_report.contains("\"return_register\":\"rax\""));
        assert!(launch_report.contains("\"name\":\"setDelegate:\""));
        assert!(launch_report.contains(
            "\"blocker\":\"return_to_continuation_objc_alloc_init_fixture_delegate_bridge_unimplemented\""
        ));
        assert!(launch_report.contains("\"unsupported_instruction\":null"));
        assert!(!launch_report.contains("\"return_to_continuation_unsupported_instruction\""));
        assert!(launch_report.contains(
            "\"next_action\":\"define_return_to_continuation_objc_alloc_init_fixture_delegate_bridge\""
        ));
        assert!(!launch_report.contains(
            "\"return_to_continuation_call_rel32_return_value_materialization_unimplemented\""
        ));
        assert!(!launch_report
            .contains("\"return_to_continuation_objc_helper_execution_unimplemented\""));
        assert!(
            !launch_report.contains("\"return_to_continuation_import_global_load_unimplemented\"")
        );
        assert!(launch_report.contains("\"error\":null"));
        assert!(
            launch_report.contains("\"schema\":\"b8_objc_message_materialization_boundary_v0\"")
        );
        assert!(launch_report.contains("\"fixup_resolution\""));
        assert!(launch_report.contains("\"symbol_name\":\"_OBJC_CLASS_$_NSApplication\""));
        assert!(launch_report.contains("\"resolved_vm_address\":4294975648"));
        assert!(!launch_report.contains("\"receiver_mapped_value_fixup_resolution_unimplemented\""));
        assert!(!launch_report.contains("\"selector_mapped_value_fixup_resolution_unimplemented\""));
        assert!(launch_report.contains("\"next_action\":\"continue_after_objc_helper_return\""));
        assert!(!launch_report.contains("\"x86_64_argument_marshaling_unimplemented\""));
        let blocker_report = read_file(&bundle_dir.join("blocker.json"));
        assert!(blocker_report.contains("\"status\":\"blocked\""));
        assert!(blocker_report.contains("\"current_blocker\":\"unsupported_instruction\""));
        assert!(blocker_report.contains("\"unsupported_instruction\":null"));
        assert!(!blocker_report.contains("DecodeUnsupportedOpcode { opcode: 85"));
        assert!(!blocker_report.contains("DecodeUnsupportedOpcode { opcode: 83"));
        assert!(!blocker_report.contains("DecodeUnsupportedOpcode { opcode: 72"));
        assert!(!blocker_report.contains("DecodeUnsupportedOpcode { opcode: 65"));
        assert!(blocker_report.contains("register_indirect_call"));
        assert!(blocker_report.contains("r14"));
        assert!(blocker_report.contains("call_site"));
        assert!(blocker_report.contains("4294972996"));
        assert!(blocker_report.contains("return_to"));
        assert!(blocker_report.contains("4294972999"));
        assert!(blocker_report
            .contains("\"next_action\":\"connect_public_rebase_bind_import_boundary\""));
        assert!(read_file(&bundle_dir.join("repro.sh")).contains("generate-b8-debug-bundle"));
    }

    #[test]
    fn compare_expected_actual_reports_matching_observations() {
        let temp_dir = TestTempDir::new("compare_expected_actual_reports_matching_observations");
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/return_42.json"),
        );
        let actual_path = temp_dir.write_file(
            "actual.json",
            include_str!("../../../tests/expected/return_42.json"),
        );

        let output = run_cli(vec![
            String::from("compare-expected-actual"),
            expected_path.to_string_lossy().into_owned(),
            actual_path.to_string_lossy().into_owned(),
        ])
        .expect("matching expected and actual JSON compare successfully");

        assert_eq!(output, "{\"issues\":[]}");
    }

    #[test]
    fn compare_expected_actual_reports_return_value_mismatch() -> Result<(), String> {
        let temp_dir = TestTempDir::new("compare_expected_actual_reports_return_value_mismatch");
        let expected_path = temp_dir.write_file(
            "expected.json",
            include_str!("../../../tests/expected/return_42.json"),
        );
        let actual_path = temp_dir.write_file(
            "actual.json",
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":41,\"stdout\":\"\",\"stderr\":\"\"}",
        );

        let error = run_cli(vec![
            String::from("compare-expected-actual"),
            expected_path.to_string_lossy().into_owned(),
            actual_path.to_string_lossy().into_owned(),
        ])
        .expect_err("mismatched expected and actual JSON fail comparison");

        let report = match error {
            CliError::Comparison(report) => report,
            other => return Err(format!("unexpected error: {other:?}")),
        };
        assert_eq!(
            report.issues(),
            &[bara_oracle::ComparisonIssue::ReturnValueMismatch {
                expected: 42,
                actual: 41,
            }]
        );

        Ok(())
    }

    #[test]
    fn comparison_report_maps_to_specific_failure_kinds() {
        assert_eq!(
            super::failure_kind_from_comparison_report(&bara_oracle::ComparisonReport::new(vec![
                bara_oracle::ComparisonIssue::ReturnValueMismatch {
                    expected: 42,
                    actual: 41,
                },
            ])),
            FailureKind::WrongRegisterValue
        );
        assert_eq!(
            super::failure_kind_from_comparison_report(&bara_oracle::ComparisonReport::new(vec![
                bara_oracle::ComparisonIssue::StdoutMismatch {
                    expected: String::from("expected"),
                    actual: String::from("actual"),
                },
            ])),
            FailureKind::WrongExternalCall
        );
        assert_eq!(
            super::failure_kind_from_comparison_report(&bara_oracle::ComparisonReport::new(vec![
                bara_oracle::ComparisonIssue::ExitStatusMismatch {
                    expected: 0,
                    actual: 42,
                },
            ])),
            FailureKind::WrongCallReturn
        );
    }

    #[test]
    fn emit_fixture_artifacts_writes_compilation_metadata_files() {
        let temp_dir = TestTempDir::new("emit_fixture_artifacts_writes_compilation_metadata_files");
        let case_path = temp_dir.write_file(
            "branch_eq_return_42.json",
            include_str!("../../../tests/cases/branch_eq_return_42.json"),
        );
        let output_dir = temp_dir.path.join("artifacts");

        let output = run_cli(vec![
            String::from("emit-fixture-artifacts"),
            case_path.to_string_lossy().into_owned(),
            output_dir.to_string_lossy().into_owned(),
        ])
        .expect("fixture artifact metadata is emitted");

        assert_eq!(
            output,
            format!(
                "{{\"compiled_ir\":\"{}\",\"pcmap\":\"{}\",\"fixups\":\"{}\",\"helpers\":\"{}\",\"artifact_report\":\"{}\",\"verifier_report\":\"{}\"}}",
                output_dir.join("compiled.ir.json").display(),
                output_dir.join("pcmap.json").display(),
                output_dir.join("fixups.json").display(),
                output_dir.join("helpers.json").display(),
                output_dir.join("artifact.report.json").display(),
                output_dir.join("verifier.report.json").display(),
            )
        );
        assert_eq!(
            read_file(&output_dir.join("compiled.ir.json")),
            "{\"entry\":0,\"blocks\":[{\"id\":0,\"start\":0,\"end\":9,\"ops\":[{\"kind\":\"mov\",\"dst\":{\"kind\":\"reg\",\"reg\":\"rax\"},\"src\":{\"kind\":\"imm_u64\",\"value\":0}},{\"kind\":\"test\",\"lhs\":{\"kind\":\"reg\",\"reg\":\"rax\"},\"rhs\":{\"kind\":\"reg\",\"reg\":\"rax\"}}],\"terminator\":{\"kind\":\"cond_jump\",\"condition\":\"equal\",\"taken\":15,\"fallthrough\":9}},{\"id\":1,\"start\":9,\"end\":15,\"ops\":[{\"kind\":\"mov\",\"dst\":{\"kind\":\"reg\",\"reg\":\"rax\"},\"src\":{\"kind\":\"imm_u64\",\"value\":7}}],\"terminator\":{\"kind\":\"return\"}},{\"id\":2,\"start\":15,\"end\":21,\"ops\":[{\"kind\":\"mov\",\"dst\":{\"kind\":\"reg\",\"reg\":\"rax\"},\"src\":{\"kind\":\"imm_u64\",\"value\":42}}],\"terminator\":{\"kind\":\"return\"}}]}"
        );
        assert_eq!(
            read_file(&output_dir.join("pcmap.json")),
            "{\"entries\":[{\"source\":0,\"target\":0},{\"source\":9,\"target\":16},{\"source\":15,\"target\":24}]}"
        );
        assert_eq!(
            read_file(&output_dir.join("fixups.json")),
            "{\"fixups\":[{\"offset\":8,\"source\":8,\"target\":15,\"kind\":{\"kind\":\"conditional\",\"condition\":\"equal\"}},{\"offset\":12,\"source\":12,\"target\":9,\"kind\":{\"kind\":\"unconditional\"}}]}"
        );
        assert_eq!(
            read_file(&output_dir.join("helpers.json")),
            "{\"helpers\":[]}"
        );
        assert_eq!(
            read_file(&output_dir.join("artifact.report.json")),
            "{\"state_layout\":{\"kind\":\"function_level_v0\",\"source_isa\":\"x86_64\",\"target_isa\":\"arm64\",\"abi\":{\"args\":[],\"return\":\"u64\"},\"return_register\":\"rax\",\"stack\":{\"kind\":\"none\"}},\"cache_validation_identity\":{\"kind\":\"fixture_function_v0\",\"case_id\":\"branch_eq_return_42\",\"source_entry\":0,\"source_bytes\":\"b80000000085c07406b807000000c3b82a000000c3\",\"source_abi\":{\"args\":[],\"return\":\"u64\"},\"target_backend\":\"bara-arm64\"},\"helper_requirements\":[]}"
        );
        assert_eq!(
            read_file(&output_dir.join("verifier.report.json")),
            "{\"issues\":[]}"
        );
    }

    #[test]
    fn emit_fixture_artifacts_report_records_stdout_helper_requirement() {
        let temp_dir =
            TestTempDir::new("emit_fixture_artifacts_report_records_stdout_helper_requirement");
        let case_path = temp_dir.write_file(
            "stdout_trap_return_0.json",
            include_str!("../../../tests/cases/stdout_trap_return_0.json"),
        );
        let output_dir = temp_dir.path.join("artifacts");

        run_cli(vec![
            String::from("emit-fixture-artifacts"),
            case_path.to_string_lossy().into_owned(),
            output_dir.to_string_lossy().into_owned(),
        ])
        .expect("fixture artifact metadata is emitted");

        assert_eq!(
            read_file(&output_dir.join("artifact.report.json")),
            "{\"state_layout\":{\"kind\":\"function_level_v0\",\"source_isa\":\"x86_64\",\"target_isa\":\"arm64\",\"abi\":{\"args\":[],\"return\":\"u64\"},\"return_register\":\"rax\",\"stack\":{\"kind\":\"none\"}},\"cache_validation_identity\":{\"kind\":\"fixture_function_v0\",\"case_id\":\"stdout_trap_return_0\",\"source_entry\":0,\"source_bytes\":\"0f0b31c0c3\",\"source_abi\":{\"args\":[],\"return\":\"u64\"},\"target_backend\":\"bara-arm64\"},\"helper_requirements\":[{\"name\":\"write_stdout\",\"signature\":\"ptr_len_to_unit\"}]}"
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn build_x86_64_macho_fixture_reports_unsupported_host() {
        let temp_dir = TestTempDir::new("build_x86_64_macho_fixture_reports_unsupported_host");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let output_path = temp_dir.path.join("return_42_x86_64");

        let error = run_cli(vec![
            String::from("build-x86_64-macho-fixture"),
            case_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect_err("non-macOS hosts cannot build x86_64 Mach-O fixtures");

        assert!(matches!(
            error,
            CliError::X8664MachOFixture(
                super::x86_64_mach_o_fixture::X8664MachOFixtureError::UnsupportedHost { .. }
            )
        ));
        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert!(!output_path.exists());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn build_x86_64_gui_hello_world_fixture_reports_unsupported_host() {
        let temp_dir =
            TestTempDir::new("build_x86_64_gui_hello_world_fixture_reports_unsupported_host");
        let output_path = temp_dir.path.join("b8_gui_hello_world_x86_64");

        let error = run_cli(vec![
            String::from("build-x86_64-gui-hello-world-fixture"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect_err("non-macOS hosts cannot build x86_64 GUI fixtures");

        assert!(matches!(
            error,
            CliError::X8664MachOFixture(
                super::x86_64_mach_o_fixture::X8664MachOFixtureError::UnsupportedHost { .. }
            )
        ));
        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert!(!output_path.exists());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn build_x86_64_gui_hello_world_visible_fixture_reports_unsupported_host() {
        let temp_dir = TestTempDir::new(
            "build_x86_64_gui_hello_world_visible_fixture_reports_unsupported_host",
        );
        let output_path = temp_dir.path.join("b8_gui_hello_world_visible_x86_64");

        let error = run_cli(vec![
            String::from("build-x86_64-gui-hello-world-visible-fixture"),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect_err("non-macOS hosts cannot build x86_64 visible GUI fixtures");

        assert!(matches!(
            error,
            CliError::X8664MachOFixture(
                super::x86_64_mach_o_fixture::X8664MachOFixtureError::UnsupportedHost { .. }
            )
        ));
        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert!(!output_path.exists());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn build_x86_64_oracle_runner_reports_unsupported_host() {
        let temp_dir = TestTempDir::new("build_x86_64_oracle_runner_reports_unsupported_host");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let output_path = temp_dir.path.join("return_42_oracle");

        let error = run_cli(vec![
            String::from("build-x86_64-oracle-runner"),
            case_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
        ])
        .expect_err("non-macOS hosts cannot build x86_64 oracle runners");

        assert!(matches!(
            error,
            CliError::X8664MachOFixture(
                super::x86_64_mach_o_fixture::X8664MachOFixtureError::UnsupportedHost { .. }
            )
        ));
        assert_eq!(error.failure_kind(), FailureKind::EmitError);
        assert!(!output_path.exists());
    }

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    #[test]
    fn generate_x86_64_expected_reports_unsupported_rosetta_host() {
        let temp_dir =
            TestTempDir::new("generate_x86_64_expected_reports_unsupported_rosetta_host");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let expected_path = temp_dir.path.join("return_42_expected.json");

        let error = run_cli(vec![
            String::from("generate-x86_64-expected"),
            case_path.to_string_lossy().into_owned(),
            expected_path.to_string_lossy().into_owned(),
        ])
        .expect_err("Rosetta expected generation requires arm64 macOS");

        assert!(matches!(
            error,
            CliError::X8664MachOFixture(
                super::x86_64_mach_o_fixture::X8664MachOFixtureError::UnsupportedRosettaHost { .. }
            )
        ));
        assert_eq!(error.failure_kind(), FailureKind::RunError);
        assert!(!expected_path.exists());
    }

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    #[test]
    fn generate_x86_64_gui_hello_world_expected_reports_unsupported_rosetta_host() {
        let temp_dir = TestTempDir::new(
            "generate_x86_64_gui_hello_world_expected_reports_unsupported_rosetta_host",
        );
        let expected_path = temp_dir.path.join("b8_gui_hello_world_expected.json");
        let launch_metadata_path = temp_dir
            .path
            .join("b8_gui_hello_world.launch.metadata.json");

        let error = run_cli(vec![
            String::from("generate-x86_64-gui-hello-world-expected"),
            expected_path.to_string_lossy().into_owned(),
            launch_metadata_path.to_string_lossy().into_owned(),
        ])
        .expect_err("Rosetta GUI expected generation requires arm64 macOS");

        assert!(matches!(
            error,
            CliError::X8664MachOFixture(
                super::x86_64_mach_o_fixture::X8664MachOFixtureError::UnsupportedRosettaHost { .. }
            )
        ));
        assert_eq!(error.failure_kind(), FailureKind::RunError);
        assert!(!expected_path.exists());
        assert!(!launch_metadata_path.exists());
    }

    #[cfg(not(all(unix, target_arch = "aarch64")))]
    #[test]
    fn generate_arm64_actual_reports_unsupported_native_runner_host() {
        let temp_dir =
            TestTempDir::new("generate_arm64_actual_reports_unsupported_native_runner_host");
        let case_path = temp_dir.write_file(
            "return_42.json",
            include_str!("../../../tests/cases/return_42.json"),
        );
        let actual_path = temp_dir.path.join("return_42_actual.json");

        let error = run_cli(vec![
            String::from("generate-arm64-actual"),
            case_path.to_string_lossy().into_owned(),
            actual_path.to_string_lossy().into_owned(),
        ])
        .expect_err("ARM64 actual generation requires an aarch64 Unix runner host");

        assert!(matches!(
            error,
            CliError::FunctionRun(super::function_run::FunctionRunError::Run(
                bara_runtime::RunError::UnsupportedHost
            ))
        ));
        assert_eq!(error.failure_kind(), FailureKind::RunError);
        assert!(!actual_path.exists());
    }

    #[test]
    fn probe_binary_reports_short_input_as_classified_error() {
        let temp_dir = TestTempDir::new("probe_binary_reports_short_input_as_classified_error");
        let binary_path = temp_dir.write_binary_file("fixture.bin", &[0xcf, 0xfa, 0xed]);

        let error = run_cli(vec![
            String::from("probe-binary"),
            binary_path.to_string_lossy().into_owned(),
        ])
        .expect_err("short binary input is classified");

        assert!(matches!(
            error,
            CliError::BinaryFormatProbe(bara_oracle::BinaryFormatProbeError::InputTooShort)
        ));
        assert_eq!(
            error.to_string(),
            "binary format probe error: InputTooShort"
        );
    }

    #[test]
    fn probe_binary_reports_unknown_magic_as_classified_error() {
        let temp_dir = TestTempDir::new("probe_binary_reports_unknown_magic_as_classified_error");
        let binary_path = temp_dir.write_binary_file("fixture.bin", &[0x00, 0x00, 0x00, 0x00]);

        let error = run_cli(vec![
            String::from("probe-binary"),
            binary_path.to_string_lossy().into_owned(),
        ])
        .expect_err("unknown binary magic is classified");

        assert!(matches!(
            error,
            CliError::BinaryFormatProbe(bara_oracle::BinaryFormatProbeError::UnknownMagic)
        ));
        assert_eq!(error.to_string(), "binary format probe error: UnknownMagic");
    }

    #[test]
    fn usage_includes_probe_binary_command() {
        let error = run_cli(Vec::new()).expect_err("missing command reports usage");

        assert!(error
            .to_string()
            .contains("check-mach-o <binary> <expected.json>"));
        assert!(error
            .to_string()
            .contains("check-mach-o-host-traps <binary> <expected.json>"));
        assert!(error
            .to_string()
            .contains("check-mach-o-host-traps <binary> <host-traps.json> <expected.json>"));
        assert!(error.to_string().contains("probe-binary <path>"));
        assert!(error
            .to_string()
            .contains("check-binary-probe <binary> <expected.json>"));
        assert!(error
            .to_string()
            .contains("emit-fixture-arm64 <case.json> <out.bin>"));
        assert!(error
            .to_string()
            .contains("link-fixture-arm64-main <case.json> <out-exe>"));
        assert!(error
            .to_string()
            .contains("build-x86_64-macho-fixture <case.json> <out-exe>"));
        assert!(error
            .to_string()
            .contains("build-x86_64-gui-hello-world-fixture <out-exe>"));
        assert!(error
            .to_string()
            .contains("build-x86_64-gui-hello-world-visible-fixture <out-exe>"));
        assert!(error
            .to_string()
            .contains("build-x86_64-oracle-runner <case.json> <out-exe>"));
        assert!(error
            .to_string()
            .contains("generate-x86_64-expected <case.json> <expected.json>"));
        assert!(error.to_string().contains(
            "generate-x86_64-gui-hello-world-expected <expected.json> <launch-metadata.json>"
        ));
        assert!(error
            .to_string()
            .contains("generate-arm64-actual <case.json> <actual.json>"));
        assert!(error.to_string().contains(
            "generate-arm64-gui-hello-world-actual <binary> <actual.json> <launch-report.json>"
        ));
        assert!(error.to_string().contains(
            "generate-arm64-gui-hello-world-translated-actual <binary> <actual.json> <launch-report.json>"
        ));
        assert!(error.to_string().contains(
            "run-arm64-gui-hello-world-translated-visible <binary> <launch-report.json>"
        ));
        assert!(error
            .to_string()
            .contains("generate-b8-debug-bundle <binary> <out-root>"));
        assert!(error
            .to_string()
            .contains("compare-expected-actual <expected.json> <actual.json>"));
        assert!(error
            .to_string()
            .contains("emit-fixture-artifacts <case.json> <out-dir>"));
        assert!(error
            .to_string()
            .contains("link-fixture-arm64-stdout-main <case.json> <out-exe>"));
        assert!(error
            .to_string()
            .contains("link-mach-o-arm64-stdout-main <binary> <out-exe>"));
        assert!(error.to_string().contains("check-blackbox [--out <dir>]"));
    }

    #[test]
    fn executable_manifest_run_result_converts_to_observed_result() {
        let manifest = bara_oracle::executable_manifest_from_json(include_str!(
            "../../../tests/executables/hello_world_executable_manifest.json"
        ))
        .expect("executable manifest parses");

        let actual = super::run_executable_manifest(&manifest)
            .expect("executable manifest runs on supported host")
            .into_observed_result();
        let expected = observed_result_from_json(include_str!(
            "../../../tests/expected/hello_world_executable_manifest.json"
        ))
        .expect("expected executable fixture parses");

        assert_eq!(actual, expected);
    }

    #[test]
    fn check_corpus_reports_all_case_json_files_in_order() {
        let temp_dir = TestTempDir::new("check_corpus_reads_all_case_json_files_in_order");
        let cases_dir = temp_dir.create_dir("cases");
        let expected_dir = temp_dir.create_dir("expected");
        write_file(
            &cases_dir.join("return_42.json"),
            include_str!("../../../tests/cases/return_42.json"),
        );
        write_file(
            &expected_dir.join("return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-corpus"),
            cases_dir.to_string_lossy().into_owned(),
            expected_dir.to_string_lossy().into_owned(),
        ])
        .expect("corpus check succeeds on supported host");

        assert_eq!(
            output,
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"}]}"
        );
    }

    #[test]
    fn check_corpus_writes_report_and_actual_outputs() {
        let temp_dir = TestTempDir::new("check_corpus_writes_report_and_actual_outputs");
        let cases_dir = temp_dir.create_dir("cases");
        let expected_dir = temp_dir.create_dir("expected");
        let output_dir = temp_dir.create_dir("out");
        write_file(
            &cases_dir.join("return_42.json"),
            include_str!("../../../tests/cases/return_42.json"),
        );
        write_file(
            &expected_dir.join("return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        );

        let output = run_cli(vec![
            String::from("check-corpus"),
            cases_dir.to_string_lossy().into_owned(),
            expected_dir.to_string_lossy().into_owned(),
            String::from("--out"),
            output_dir.to_string_lossy().into_owned(),
        ])
        .expect("corpus check succeeds on supported host");

        assert_eq!(
            output,
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"}]}"
        );
        assert_eq!(
            read_file(&output_dir.join("report.json")),
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"}]}"
        );
        assert_eq!(
            read_file(&output_dir.join("actual").join("return_42.json")),
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("compiled")
                    .join("return_42")
                    .join("artifact.report.json")
            ),
            "{\"state_layout\":{\"kind\":\"function_level_v0\",\"source_isa\":\"x86_64\",\"target_isa\":\"arm64\",\"abi\":{\"args\":[],\"return\":\"u64\"},\"return_register\":\"rax\",\"stack\":{\"kind\":\"none\"}},\"cache_validation_identity\":{\"kind\":\"fixture_function_v0\",\"case_id\":\"return_42\",\"source_entry\":0,\"source_bytes\":\"b82a000000c3\",\"source_abi\":{\"args\":[],\"return\":\"u64\"},\"target_backend\":\"bara-arm64\"},\"helper_requirements\":[]}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("compiled")
                    .join("return_42")
                    .join("compiled.ir.json")
            ),
            "{\"entry\":0,\"blocks\":[{\"id\":0,\"start\":0,\"end\":6,\"ops\":[{\"kind\":\"mov\",\"dst\":{\"kind\":\"reg\",\"reg\":\"rax\"},\"src\":{\"kind\":\"imm_u64\",\"value\":42}}],\"terminator\":{\"kind\":\"return\"}}]}"
        );
    }

    #[test]
    fn check_blackbox_reports_raw_manifest_mach_o_and_probe_fixtures() {
        let output = run_cli(vec![String::from("check-blackbox")])
            .expect("blackbox check succeeds on supported host");

        assert_eq!(output, expected_blackbox_report_json());
    }

    #[test]
    fn check_blackbox_writes_report_and_schema_specific_actual_outputs() {
        let temp_dir =
            TestTempDir::new("check_blackbox_writes_report_and_schema_specific_actual_outputs");
        let output_dir = temp_dir.create_dir("out");

        let output = run_cli(vec![
            String::from("check-blackbox"),
            String::from("--out"),
            output_dir.to_string_lossy().into_owned(),
        ])
        .expect("blackbox check succeeds on supported host");

        assert_eq!(output, expected_blackbox_report_json());
        assert_eq!(
            read_file(&output_dir.join("report.json")),
            expected_blackbox_report_json()
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("hello_world_executable_manifest.json")
            ),
            "{\"case_id\":\"hello_world_executable_manifest\",\"exit_status\":0,\"return_value\":0,\"stdout\":\"hello world\\n\",\"stderr\":\"\"}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("mach_o_hello_world_stdout.json")
            ),
            "{\"case_id\":\"mach_o_hello_world_stdout\",\"exit_status\":0,\"return_value\":0,\"stdout\":\"hello world\\n\",\"stderr\":\"\"}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("mach_o_hello_world_stdout_native_executable.json")
            ),
            "{\"case_id\":\"mach_o_hello_world_stdout_native_executable\",\"exit_status\":0,\"return_value\":0,\"stdout\":\"hello world\\n\",\"stderr\":\"\"}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("return_42_native_executable_smoke.json")
            ),
            "{\"case_id\":\"return_42_native_executable_smoke\",\"exit_status\":42,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("mach_o_return_42_native_executable_smoke.json")
            ),
            "{\"case_id\":\"mach_o_return_42_native_executable_smoke\",\"exit_status\":42,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}"
        );
        assert_eq!(
            read_file(
                &output_dir
                    .join("compiled")
                    .join("stdout_trap_return_0")
                    .join("artifact.report.json")
            ),
            "{\"state_layout\":{\"kind\":\"function_level_v0\",\"source_isa\":\"x86_64\",\"target_isa\":\"arm64\",\"abi\":{\"args\":[],\"return\":\"u64\"},\"return_register\":\"rax\",\"stack\":{\"kind\":\"none\"}},\"cache_validation_identity\":{\"kind\":\"fixture_function_v0\",\"case_id\":\"stdout_trap_return_0\",\"source_entry\":0,\"source_bytes\":\"0f0b31c0c3\",\"source_abi\":{\"args\":[],\"return\":\"u64\"},\"target_backend\":\"bara-arm64\"},\"helper_requirements\":[{\"name\":\"write_stdout\",\"signature\":\"ptr_len_to_unit\"}]}"
        );
        assert!(output_dir
            .join("native-artifacts")
            .join("mach_o_return_42_native_executable_smoke")
            .is_file());
        assert!(output_dir
            .join("native-artifacts")
            .join("mach_o_hello_world_stdout_native_executable")
            .is_file());
        let expected_probe = binary_format_probe_report_from_json(include_str!(
            "../../../tests/expected-probes/mach_o_execute_header.json"
        ))
        .and_then(|report| binary_format_probe_report_to_json(&report))
        .expect("expected probe report normalizes to output json");
        assert_eq!(
            read_file(
                &output_dir
                    .join("actual")
                    .join("mach_o_execute_header_probe.json")
            ),
            expected_probe
        );
    }

    #[test]
    fn check_corpus_continues_after_failed_case() -> Result<(), String> {
        let temp_dir = TestTempDir::new("check_corpus_continues_after_failed_case");
        let cases_dir = temp_dir.create_dir("cases");
        let expected_dir = temp_dir.create_dir("expected");
        write_file(
            &cases_dir.join("bad_hex.json"),
            r#"{"case_id":"bad_hex","entry":0,"bytes":"cg","abi":{"args":[],"return":"u64"}}"#,
        );
        write_file(
            &cases_dir.join("return_42.json"),
            include_str!("../../../tests/cases/return_42.json"),
        );
        write_file(
            &expected_dir.join("return_42.json"),
            include_str!("../../../tests/expected/return_42.json"),
        );

        let error = run_cli(vec![
            String::from("check-corpus"),
            cases_dir.to_string_lossy().into_owned(),
            expected_dir.to_string_lossy().into_owned(),
        ])
        .expect_err("corpus check reports failures after scanning every case");

        let report = match error {
            CliError::CorpusFailures(report) => report,
            other => return Err(format!("unexpected error: {other:?}")),
        };

        assert!(!report.is_success());
        assert_eq!(report.fixtures().len(), 2);
        assert_eq!(report.fixtures()[0].case_id().as_str(), "bad_hex");
        assert_eq!(
            report.fixtures()[0].outcome(),
            &FixtureOutcome::failed(
                FailureKind::InvalidTestCase,
                FailureMessage::from("invalid hex digit at index 1")
            )
        );
        assert_eq!(report.fixtures()[1].case_id().as_str(), "return_42");
        assert_eq!(report.fixtures()[1].outcome(), &FixtureOutcome::Passed);

        Ok(())
    }

    #[test]
    fn check_corpus_classifies_return_value_mismatch_as_wrong_register() -> Result<(), String> {
        let temp_dir =
            TestTempDir::new("check_corpus_classifies_return_value_mismatch_as_wrong_register");
        let cases_dir = temp_dir.create_dir("cases");
        let expected_dir = temp_dir.create_dir("expected");
        let output_dir = temp_dir.create_dir("out");
        let testcase_json = include_str!("../../../tests/cases/return_42.json");
        let expected_json =
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":41,\"stdout\":\"\",\"stderr\":\"\"}";
        write_file(&cases_dir.join("return_42.json"), testcase_json);
        write_file(&expected_dir.join("return_42.json"), expected_json);

        let error = run_cli(vec![
            String::from("check-corpus"),
            cases_dir.to_string_lossy().into_owned(),
            expected_dir.to_string_lossy().into_owned(),
            String::from("--out"),
            output_dir.to_string_lossy().into_owned(),
        ])
        .expect_err("corpus check reports comparison mismatch");

        let report = match error {
            CliError::CorpusFailures(report) => report,
            other => return Err(format!("unexpected error: {other:?}")),
        };
        assert!(!report.is_success());

        let failure_dir = output_dir.join("failures").join("return_42");
        assert_eq!(
            read_file(&failure_dir.join("failure.json")),
            "{\"case_id\":\"return_42\",\"kind\":\"wrong_register_value\",\"message\":\"comparison failed: ComparisonReport { issues: [ReturnValueMismatch { expected: 41, actual: 42 }] }\",\"final_state\":{\"issues\":[{\"return_value_mismatch\":{\"expected\":41,\"actual\":42}}]},\"shrink\":{\"status\":\"not_attempted\",\"recommended_next_step\":\"minimize testcase while preserving failure kind wrong_register_value\"},\"corpus_update\":{\"action\":\"review_failure_package\",\"candidate_testcase\":\"testcase.json\",\"candidate_expected\":\"expected.json\",\"candidate_actual\":\"actual.json\"}}"
        );
        assert_eq!(read_file(&failure_dir.join("testcase.json")), testcase_json);
        assert_eq!(read_file(&failure_dir.join("expected.json")), expected_json);
        assert_eq!(
            read_file(&failure_dir.join("actual.json")),
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}"
        );

        Ok(())
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

        fn create_dir(&self, name: &str) -> PathBuf {
            let path = self.path.join(name);
            fs::create_dir(&path).expect("test child dir is created");
            path
        }

        fn write_file(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.path.join(name);
            write_file(&path, contents);
            path
        }

        fn write_binary_file(&self, name: &str, contents: &[u8]) -> PathBuf {
            let path = self.path.join(name);
            fs::write(&path, contents).expect("test binary fixture file is written");
            path
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).expect("test temp dir is removed");
        }
    }

    fn write_file(path: &Path, contents: &str) {
        fs::write(path, contents).expect("test fixture file is written");
    }

    fn read_file(path: &Path) -> String {
        fs::read_to_string(path).expect("test fixture file is read")
    }

    fn expected_blackbox_report_json() -> &'static str {
        "{\"fixtures\":[{\"case_id\":\"add_eax_imm32_return_45\",\"outcome\":\"passed\"},{\"case_id\":\"add_eax_imm_return_45\",\"outcome\":\"passed\"},{\"case_id\":\"add_sub_eax_imm_return_40\",\"outcome\":\"passed\"},{\"case_id\":\"branch_eq_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"direct_jmp_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"hello_world_stdout_return_0\",\"outcome\":\"passed\"},{\"case_id\":\"identity_u64\",\"outcome\":\"passed\"},{\"case_id\":\"jl_rel32_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"load_u8_from_rdi_return_72\",\"outcome\":\"passed\"},{\"case_id\":\"loop_countdown_return_0\",\"outcome\":\"passed\"},{\"case_id\":\"nested_call_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"push_pop_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"return_42\",\"outcome\":\"passed\"},{\"case_id\":\"stdout_trap_return_0\",\"outcome\":\"passed\"},{\"case_id\":\"sub_eax_imm32_return_39\",\"outcome\":\"passed\"},{\"case_id\":\"sub_eax_imm_return_39\",\"outcome\":\"passed\"},{\"case_id\":\"xor_eax_eax_return_0\",\"outcome\":\"passed\"},{\"case_id\":\"xor_then_add_eax_return_7\",\"outcome\":\"passed\"},{\"case_id\":\"return_42_native_executable_smoke\",\"outcome\":\"passed\"},{\"case_id\":\"hello_world_executable_manifest\",\"outcome\":\"passed\"},{\"case_id\":\"entry_offset_return_42_manifest\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_return_42\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_return_42_native_executable_smoke\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_hello_world_stdout\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_hello_world_stdout_native_executable\",\"outcome\":\"passed\"},{\"case_id\":\"mach_o_execute_header_probe\",\"outcome\":\"passed\"}]}"
    }
}
