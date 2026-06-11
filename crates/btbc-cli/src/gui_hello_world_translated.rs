use std::{error::Error, fmt};

use bara_ir::X86Va;
use bara_isa_x86::{DecodeError, X86Bytes};
use bara_oracle::{BinaryFormatProbeReport, CaseId, CaseIdError, ObservedResult, TestCase};
use serde::Serialize;

use crate::{
    function_run::{
        run_test_case_function_with_bundle, FunctionArtifactMetadata, FunctionRunError,
    },
    x86_64_mach_o_fixture::{b8_gui_hello_world_case_id, X8664MachOFixtureError},
};

pub(crate) const B8_GUI_TRANSLATED_ENTRY_CASE_ID: &str = "b8_gui_hello_world_translated_entry";
pub(crate) const B8_GUI_TRANSLATED_ENTRY_BYTES: &[u8] =
    &[0x0f, 0x0b, b'B', b'8', b'G', b'1', 0x31, 0xc0, 0xc3];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GuiHelloWorldTranslatedLaunchMode {
    AutomatedOracle,
    ManualVisible,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GuiHelloWorldTranslatedLaunchBundle {
    observed_result: ObservedResult,
    launch_report: GuiHelloWorldTranslatedLaunchReport,
}

impl GuiHelloWorldTranslatedLaunchBundle {
    pub(crate) const fn observed_result(&self) -> &ObservedResult {
        &self.observed_result
    }

    pub(crate) const fn launch_report(&self) -> &GuiHelloWorldTranslatedLaunchReport {
        &self.launch_report
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct GuiHelloWorldTranslatedLaunchReport {
    schema: &'static str,
    case_id: CaseId,
    mode: GuiHelloWorldTranslatedLaunchMode,
    status: GuiHelloWorldTranslatedLaunchStatus,
    input_probe: BinaryFormatProbeReport,
    translated_entry: GuiHelloWorldTranslatedEntryReport,
    helper_capability: GuiHelloWorldTranslatedHelperCapabilityReport,
    launch_result: GuiHelloWorldTranslatedLaunchResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GuiHelloWorldTranslatedLaunchStatus {
    GuiVisibleReady,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldTranslatedEntryReport {
    source: GuiHelloWorldTranslatedEntrySource,
    pipeline: GuiHelloWorldTranslatedEntryPipeline,
    artifact_metadata: FunctionArtifactMetadata,
    host_trap_request: GuiHelloWorldTranslatedHostTrapRequest,
    runtime_result: GuiHelloWorldTranslatedRuntimeResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldTranslatedEntrySource {
    kind: GuiHelloWorldTranslatedEntrySourceKind,
    source_isa: GuiHelloWorldTranslatedSourceIsa,
    source_bytes: &'static str,
}

impl GuiHelloWorldTranslatedEntrySource {
    const fn b8_g1() -> Self {
        Self {
            kind: GuiHelloWorldTranslatedEntrySourceKind::BaraGuiHostTrapEntryV0,
            source_isa: GuiHelloWorldTranslatedSourceIsa::X8664,
            source_bytes: "0f0b4238473131c0c3",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GuiHelloWorldTranslatedEntrySourceKind {
    BaraGuiHostTrapEntryV0,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldTranslatedSourceIsa {
    #[serde(rename = "x86_64")]
    X8664,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldTranslatedEntryPipeline {
    decode: GuiHelloWorldTranslatedPipelineStageStatus,
    lift: GuiHelloWorldTranslatedPipelineStageStatus,
    emit: GuiHelloWorldTranslatedPipelineStageStatus,
    runtime: GuiHelloWorldTranslatedPipelineStageStatus,
}

impl GuiHelloWorldTranslatedEntryPipeline {
    const fn executed() -> Self {
        Self {
            decode: GuiHelloWorldTranslatedPipelineStageStatus::Executed,
            lift: GuiHelloWorldTranslatedPipelineStageStatus::Executed,
            emit: GuiHelloWorldTranslatedPipelineStageStatus::Executed,
            runtime: GuiHelloWorldTranslatedPipelineStageStatus::Executed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GuiHelloWorldTranslatedPipelineStageStatus {
    Executed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldTranslatedHostTrapRequest {
    kind: GuiHelloWorldTranslatedHostTrapKind,
    requested: bool,
}

impl GuiHelloWorldTranslatedHostTrapRequest {
    const fn appkit_gui_hello_world(requested: bool) -> Self {
        Self {
            kind: GuiHelloWorldTranslatedHostTrapKind::AppKitGuiHelloWorld,
            requested,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GuiHelloWorldTranslatedHostTrapKind {
    #[serde(rename = "appkit_gui_hello_world")]
    AppKitGuiHelloWorld,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldTranslatedRuntimeResult {
    return_value: u64,
    stdout: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldTranslatedHelperCapabilityReport {
    contract: GuiHelloWorldTranslatedHelperContract,
    invoked_by: GuiHelloWorldTranslatedHelperInvocationSource,
    run_mode: GuiHelloWorldTranslatedLaunchMode,
    status: GuiHelloWorldTranslatedHelperStatus,
}

impl GuiHelloWorldTranslatedHelperCapabilityReport {
    const fn executed(run_mode: GuiHelloWorldTranslatedLaunchMode) -> Self {
        Self {
            contract: GuiHelloWorldTranslatedHelperContract::AppKitGuiLifecycleEvent,
            invoked_by: GuiHelloWorldTranslatedHelperInvocationSource::TranslatedHostTrapRequest,
            run_mode,
            status: GuiHelloWorldTranslatedHelperStatus::Executed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GuiHelloWorldTranslatedHelperContract {
    #[serde(rename = "appkit_gui_lifecycle_event")]
    AppKitGuiLifecycleEvent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GuiHelloWorldTranslatedHelperInvocationSource {
    TranslatedHostTrapRequest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GuiHelloWorldTranslatedHelperStatus {
    Executed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldTranslatedLaunchResult {
    exit_status: i32,
    return_value: u64,
    stdout: String,
    stderr: String,
}

impl GuiHelloWorldTranslatedLaunchResult {
    fn from_observed_result(result: &ObservedResult) -> Self {
        Self {
            exit_status: result.exit_status(),
            return_value: result.return_value(),
            stdout: result.stdout().to_owned(),
            stderr: result.stderr().to_owned(),
        }
    }
}

pub(crate) fn b8_gui_hello_world_translated_launch(
    input_probe: BinaryFormatProbeReport,
    helper_result: ObservedResult,
    mode: GuiHelloWorldTranslatedLaunchMode,
) -> Result<GuiHelloWorldTranslatedLaunchBundle, GuiHelloWorldTranslatedLaunchError> {
    let test_case = translated_entry_test_case()?;
    let translated_entry = run_test_case_function_with_bundle(&test_case)
        .map_err(GuiHelloWorldTranslatedLaunchError::FunctionRun)?;
    let appkit_request = translated_entry
        .compiled()
        .appkit_gui_hello_world_host_trap_request();
    if !appkit_request.is_requested() {
        return Err(GuiHelloWorldTranslatedLaunchError::MissingAppKitGuiRequest);
    }
    if translated_entry.result().return_value() != 0 {
        return Err(
            GuiHelloWorldTranslatedLaunchError::TranslatedEntryNonZeroReturn {
                return_value: translated_entry.result().return_value(),
            },
        );
    }

    let case_id =
        b8_gui_hello_world_case_id().map_err(GuiHelloWorldTranslatedLaunchError::B8CaseId)?;
    let launch_result = GuiHelloWorldTranslatedLaunchResult::from_observed_result(&helper_result);
    let launch_report = GuiHelloWorldTranslatedLaunchReport {
        schema: "b8_gui_hello_world_translated_launch_report_v0",
        case_id,
        mode,
        status: GuiHelloWorldTranslatedLaunchStatus::GuiVisibleReady,
        input_probe,
        translated_entry: GuiHelloWorldTranslatedEntryReport {
            source: GuiHelloWorldTranslatedEntrySource::b8_g1(),
            pipeline: GuiHelloWorldTranslatedEntryPipeline::executed(),
            artifact_metadata: translated_entry.compiled().artifact_metadata(&test_case),
            host_trap_request: GuiHelloWorldTranslatedHostTrapRequest::appkit_gui_hello_world(
                appkit_request.is_requested(),
            ),
            runtime_result: GuiHelloWorldTranslatedRuntimeResult {
                return_value: translated_entry.result().return_value(),
                stdout: translated_entry.result().stdout().to_owned(),
            },
        },
        helper_capability: GuiHelloWorldTranslatedHelperCapabilityReport::executed(mode),
        launch_result,
    };

    Ok(GuiHelloWorldTranslatedLaunchBundle {
        observed_result: helper_result,
        launch_report,
    })
}

pub(crate) fn translated_entry_test_case() -> Result<TestCase, GuiHelloWorldTranslatedLaunchError> {
    let case_id = CaseId::new(B8_GUI_TRANSLATED_ENTRY_CASE_ID)
        .map_err(GuiHelloWorldTranslatedLaunchError::TranslatedEntryCaseId)?;
    let bytes = X86Bytes::new(X86Va::new(0), B8_GUI_TRANSLATED_ENTRY_BYTES.to_vec())
        .map_err(GuiHelloWorldTranslatedLaunchError::TranslatedEntryBytes)?;

    Ok(TestCase::new(
        case_id,
        bytes,
        bara_oracle::TestCaseAbi::NoArgsU64,
    ))
}

#[derive(Debug)]
pub(crate) enum GuiHelloWorldTranslatedLaunchError {
    TranslatedEntryCaseId(CaseIdError),
    TranslatedEntryBytes(DecodeError),
    FunctionRun(FunctionRunError),
    MissingAppKitGuiRequest,
    TranslatedEntryNonZeroReturn { return_value: u64 },
    B8CaseId(X8664MachOFixtureError),
}

impl fmt::Display for GuiHelloWorldTranslatedLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TranslatedEntryCaseId(error) => {
                write!(formatter, "invalid B8 translated entry case id: {error:?}")
            }
            Self::TranslatedEntryBytes(error) => {
                write!(formatter, "invalid B8 translated entry bytes: {error:?}")
            }
            Self::FunctionRun(error) => {
                write!(formatter, "B8 translated entry run failed: {error}")
            }
            Self::MissingAppKitGuiRequest => write!(
                formatter,
                "B8 translated entry did not request the AppKit GUI helper capability"
            ),
            Self::TranslatedEntryNonZeroReturn { return_value } => write!(
                formatter,
                "B8 translated entry returned non-zero value {return_value}"
            ),
            Self::B8CaseId(error) => write!(formatter, "invalid B8 GUI case id: {error}"),
        }
    }
}

impl Error for GuiHelloWorldTranslatedLaunchError {}

#[cfg(test)]
mod tests {
    use bara_oracle::{
        probe_public_binary_format, BinaryFileBytes, BinaryInput, CaseId, ObservedResult,
    };

    use super::{b8_gui_hello_world_translated_launch, GuiHelloWorldTranslatedLaunchMode};

    #[test]
    fn translated_launch_report_records_runtime_path_and_appkit_helper_request() {
        let input = BinaryInput::from_file_bytes(BinaryFileBytes::from_untrusted_file_contents(
            include_bytes!("../../../tests/binaries/mach_o_execute_header.bin").to_vec(),
        ));
        let probe = probe_public_binary_format(&input).expect("minimal Mach-O fixture probes");
        let helper_result = ObservedResult::new(
            CaseId::new("b8_gui_hello_world").expect("case id is non-empty"),
            0,
            0,
            "{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}\n".to_owned(),
            String::new(),
        );

        let bundle = b8_gui_hello_world_translated_launch(
            probe,
            helper_result,
            GuiHelloWorldTranslatedLaunchMode::AutomatedOracle,
        )
        .expect("translated B8 GUI launch report builds");
        let report = serde_json::to_string(bundle.launch_report()).expect("report serializes");

        assert!(report.contains("\"schema\":\"b8_gui_hello_world_translated_launch_report_v0\""));
        assert!(report.contains("\"source_bytes\":\"0f0b4238473131c0c3\""));
        assert!(report.contains("\"kind\":\"appkit_gui_hello_world\",\"requested\":true"));
        assert!(report.contains("\"runtime\":\"executed\""));
        assert!(report.contains("\"invoked_by\":\"translated_host_trap_request\""));
        assert_eq!(bundle.observed_result().stdout(), "{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}\n");
    }
}
