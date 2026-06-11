use bara_oracle::{CaseId, ObservedResult};
use serde::Serialize;

use crate::x86_64_mach_o_fixture::{b8_gui_hello_world_case_id, X8664MachOFixtureError};

const B8_GUI_HELLO_WORLD_ACTUAL_STDERR: &str = "unsupported_boundary: unsupported_loader_feature";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GuiHelloWorldActualLaunchBundle {
    observed_result: ObservedResult,
    launch_report: GuiHelloWorldActualLaunchReport,
}

impl GuiHelloWorldActualLaunchBundle {
    fn blocked_by_loader(case_id: CaseId) -> Self {
        let observed_result = ObservedResult::new(
            case_id.clone(),
            1,
            0,
            String::new(),
            String::from(B8_GUI_HELLO_WORLD_ACTUAL_STDERR),
        );
        let launch_report = GuiHelloWorldActualLaunchReport::blocked_by_loader(case_id);

        Self {
            observed_result,
            launch_report,
        }
    }

    pub(crate) const fn observed_result(&self) -> &ObservedResult {
        &self.observed_result
    }

    pub(crate) const fn launch_report(&self) -> &GuiHelloWorldActualLaunchReport {
        &self.launch_report
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct GuiHelloWorldActualLaunchReport {
    schema: &'static str,
    case_id: CaseId,
    actual_runtime: GuiHelloWorldActualRuntime,
    status: GuiHelloWorldActualLaunchStatus,
    input: GuiHelloWorldActualInput,
    blocker: GuiHelloWorldActualBlocker,
}

impl GuiHelloWorldActualLaunchReport {
    fn blocked_by_loader(case_id: CaseId) -> Self {
        Self {
            schema: "b8_gui_hello_world_actual_launch_report_v0",
            case_id,
            actual_runtime: GuiHelloWorldActualRuntime::BaraArm64UserSpace,
            status: GuiHelloWorldActualLaunchStatus::Blocked,
            input: GuiHelloWorldActualInput::new(),
            blocker: GuiHelloWorldActualBlocker::unsupported_loader_feature(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualRuntime {
    #[serde(rename = "bara_arm64_user_space")]
    BaraArm64UserSpace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLaunchStatus {
    #[serde(rename = "blocked")]
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualInput {
    kind: GuiHelloWorldActualInputKind,
    source_isa: GuiHelloWorldActualSourceIsa,
    binary_format: GuiHelloWorldActualBinaryFormat,
    target_triple: GuiHelloWorldActualTargetTriple,
    gui_framework: GuiHelloWorldActualFramework,
}

impl GuiHelloWorldActualInput {
    const fn new() -> Self {
        Self {
            kind: GuiHelloWorldActualInputKind::SingleMachOExecutable,
            source_isa: GuiHelloWorldActualSourceIsa::X8664,
            binary_format: GuiHelloWorldActualBinaryFormat::MachO,
            target_triple: GuiHelloWorldActualTargetTriple::X8664AppleMacos13,
            gui_framework: GuiHelloWorldActualFramework::AppKit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualInputKind {
    #[serde(rename = "single_mach_o_executable")]
    SingleMachOExecutable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualSourceIsa {
    #[serde(rename = "x86_64")]
    X8664,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBinaryFormat {
    #[serde(rename = "mach_o")]
    MachO,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualTargetTriple {
    #[serde(rename = "x86_64-apple-macos13")]
    X8664AppleMacos13,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualFramework {
    #[serde(rename = "appkit")]
    AppKit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualBlocker {
    classification: GuiHelloWorldActualBlockerClassification,
    message: &'static str,
}

impl GuiHelloWorldActualBlocker {
    const fn unsupported_loader_feature() -> Self {
        Self {
            classification: GuiHelloWorldActualBlockerClassification::UnsupportedLoaderFeature,
            message: "Bara does not yet load a complete x86_64 Mach-O GUI executable with dynamic loader, AppKit import, and Objective-C runtime requirements.",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBlockerClassification {
    #[serde(rename = "unsupported_loader_feature")]
    UnsupportedLoaderFeature,
}

pub(crate) fn b8_gui_hello_world_actual_launch_attempt(
) -> Result<GuiHelloWorldActualLaunchBundle, X8664MachOFixtureError> {
    Ok(GuiHelloWorldActualLaunchBundle::blocked_by_loader(
        b8_gui_hello_world_case_id()?,
    ))
}

#[cfg(test)]
mod tests {
    use bara_oracle::{CaseId, ObservedResult};

    use super::b8_gui_hello_world_actual_launch_attempt;

    #[test]
    fn gui_hello_world_actual_attempt_reports_loader_blocker() {
        let attempt = b8_gui_hello_world_actual_launch_attempt()
            .expect("built-in B8 GUI Hello World case id is valid");

        assert_eq!(
            attempt.observed_result(),
            &ObservedResult::new(
                CaseId::new("b8_gui_hello_world").expect("case id is non-empty"),
                1,
                0,
                String::new(),
                String::from("unsupported_boundary: unsupported_loader_feature"),
            )
        );
        assert_eq!(
            serde_json::to_string(attempt.launch_report()).expect("launch report serializes"),
            "{\"schema\":\"b8_gui_hello_world_actual_launch_report_v0\",\"case_id\":\"b8_gui_hello_world\",\"actual_runtime\":\"bara_arm64_user_space\",\"status\":\"blocked\",\"input\":{\"kind\":\"single_mach_o_executable\",\"source_isa\":\"x86_64\",\"binary_format\":\"mach_o\",\"target_triple\":\"x86_64-apple-macos13\",\"gui_framework\":\"appkit\"},\"blocker\":{\"classification\":\"unsupported_loader_feature\",\"message\":\"Bara does not yet load a complete x86_64 Mach-O GUI executable with dynamic loader, AppKit import, and Objective-C runtime requirements.\"}}"
        );
    }
}
