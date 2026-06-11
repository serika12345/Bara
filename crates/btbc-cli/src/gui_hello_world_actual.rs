use bara_oracle::{
    BinaryFormat, BinaryFormatProbeReport, BinaryFormatProbeStatus, CaseId, MachOMetadata,
    ObservedResult,
};
use serde::Serialize;

use crate::x86_64_mach_o_fixture::{b8_gui_hello_world_case_id, X8664MachOFixtureError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GuiHelloWorldActualLaunchBundle {
    observed_result: ObservedResult,
    launch_report: GuiHelloWorldActualLaunchReport,
}

impl GuiHelloWorldActualLaunchBundle {
    fn blocked_by_initial_blocker(
        case_id: CaseId,
        input_metadata: GuiHelloWorldActualInputMetadata,
        classification_plan: GuiHelloWorldInitialBlockerPlan,
    ) -> Self {
        let classification = classification_plan.selected_classification();
        let observed_result = ObservedResult::new(
            case_id.clone(),
            1,
            0,
            String::new(),
            String::from(classification.stderr_message()),
        );
        let launch_report = GuiHelloWorldActualLaunchReport::blocked_by_initial_blocker(
            case_id,
            input_metadata,
            classification_plan,
        );

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
    fn blocked_by_initial_blocker(
        case_id: CaseId,
        input_metadata: GuiHelloWorldActualInputMetadata,
        classification_plan: GuiHelloWorldInitialBlockerPlan,
    ) -> Self {
        Self {
            schema: "b8_gui_hello_world_actual_launch_report_v0",
            case_id,
            actual_runtime: GuiHelloWorldActualRuntime::BaraArm64UserSpace,
            status: GuiHelloWorldActualLaunchStatus::Blocked,
            input: GuiHelloWorldActualInput::from_metadata(input_metadata),
            blocker: GuiHelloWorldActualBlocker::from_classification_plan(&classification_plan),
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
    probe: GuiHelloWorldActualInputProbe,
    loader_metadata: GuiHelloWorldActualLoaderMetadata,
}

impl GuiHelloWorldActualInput {
    fn from_metadata(metadata: GuiHelloWorldActualInputMetadata) -> Self {
        Self {
            kind: GuiHelloWorldActualInputKind::MachOExecutableImage,
            source_isa: GuiHelloWorldActualSourceIsa::X8664,
            binary_format: GuiHelloWorldActualBinaryFormat::MachO,
            target_triple: GuiHelloWorldActualTargetTriple::X8664AppleMacos13,
            gui_framework: GuiHelloWorldActualFramework::AppKit,
            probe: metadata.probe,
            loader_metadata: metadata.loader_metadata,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualInputKind {
    #[serde(rename = "mach_o_executable_image")]
    MachOExecutableImage,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualInputProbe {
    format: BinaryFormat,
    status: BinaryFormatProbeStatus,
}

impl GuiHelloWorldActualInputProbe {
    const fn from_report(report: &BinaryFormatProbeReport) -> Self {
        Self {
            format: report.format(),
            status: report.status(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualInputMetadata {
    probe: GuiHelloWorldActualInputProbe,
    loader_metadata: GuiHelloWorldActualLoaderMetadata,
}

impl GuiHelloWorldActualInputMetadata {
    fn from_report(report: &BinaryFormatProbeReport) -> Self {
        Self {
            probe: GuiHelloWorldActualInputProbe::from_report(report),
            loader_metadata: GuiHelloWorldActualLoaderMetadata::from_report(report),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualLoaderMetadata {
    source: GuiHelloWorldActualLoaderMetadataSource,
    mach_o: MachOMetadata,
    sections: GuiHelloWorldActualDeferredLoaderMetadata,
    imports: GuiHelloWorldActualDeferredLoaderMetadata,
    relocations: GuiHelloWorldActualDeferredLoaderMetadata,
}

impl GuiHelloWorldActualLoaderMetadata {
    fn from_report(report: &BinaryFormatProbeReport) -> Self {
        Self {
            source: GuiHelloWorldActualLoaderMetadataSource::PublicMachOProbe,
            mach_o: report.metadata().mach_o_metadata().clone(),
            sections:
                GuiHelloWorldActualDeferredLoaderMetadata::modeled_from_lc_segment_64_section_table(
                ),
            imports: GuiHelloWorldActualDeferredLoaderMetadata::not_modeled(),
            relocations: GuiHelloWorldActualDeferredLoaderMetadata::not_modeled(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualLoaderMetadataSource {
    #[serde(rename = "public_mach_o_probe")]
    PublicMachOProbe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualDeferredLoaderMetadata {
    status: GuiHelloWorldActualDeferredLoaderMetadataStatus,
}

impl GuiHelloWorldActualDeferredLoaderMetadata {
    const fn modeled_from_lc_segment_64_section_table() -> Self {
        Self {
            status:
                GuiHelloWorldActualDeferredLoaderMetadataStatus::ModeledFromLcSegment64SectionTable,
        }
    }

    const fn not_modeled() -> Self {
        Self {
            status: GuiHelloWorldActualDeferredLoaderMetadataStatus::NotModeled,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualDeferredLoaderMetadataStatus {
    #[serde(rename = "modeled_from_lc_segment_64_section_table")]
    ModeledFromLcSegment64SectionTable,
    #[serde(rename = "not_modeled")]
    NotModeled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualBlocker {
    classification: GuiHelloWorldActualBlockerClassification,
    boundary: GuiHelloWorldUnsupportedLaunchBoundary,
    selected_by: GuiHelloWorldActualBlockerSelectionRule,
    candidate_boundaries: Vec<GuiHelloWorldActualBlockerCandidate>,
    message: &'static str,
}

impl GuiHelloWorldActualBlocker {
    fn from_classification_plan(plan: &GuiHelloWorldInitialBlockerPlan) -> Self {
        let boundary = plan.selected_boundary();
        Self {
            classification: boundary.classification(),
            boundary,
            selected_by: GuiHelloWorldActualBlockerSelectionRule::FirstUnsupportedLaunchBoundary,
            candidate_boundaries: plan.candidate_boundaries(),
            message: boundary.message(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBlockerClassification {
    #[serde(rename = "unsupported_import")]
    Import,
    #[serde(rename = "unsupported_loader_feature")]
    LoaderFeature,
    #[serde(rename = "unsupported_objc_runtime_boundary")]
    ObjcRuntimeBoundary,
}

impl GuiHelloWorldActualBlockerClassification {
    const fn stderr_message(self) -> &'static str {
        match self {
            Self::Import => "unsupported_boundary: unsupported_import",
            Self::LoaderFeature => "unsupported_boundary: unsupported_loader_feature",
            Self::ObjcRuntimeBoundary => "unsupported_boundary: unsupported_objc_runtime_boundary",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldUnsupportedLaunchBoundary {
    #[serde(rename = "import")]
    Import,
    #[serde(rename = "loader")]
    Loader,
    #[serde(rename = "objc_runtime")]
    ObjcRuntime,
}

impl GuiHelloWorldUnsupportedLaunchBoundary {
    const fn classification(self) -> GuiHelloWorldActualBlockerClassification {
        match self {
            Self::Import => GuiHelloWorldActualBlockerClassification::Import,
            Self::Loader => GuiHelloWorldActualBlockerClassification::LoaderFeature,
            Self::ObjcRuntime => GuiHelloWorldActualBlockerClassification::ObjcRuntimeBoundary,
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::Import => {
                "Bara does not yet resolve the GUI fixture's public AppKit import boundary."
            }
            Self::Loader => {
                "Bara does not yet load a complete x86_64 Mach-O GUI executable with dynamic loader, AppKit import, and Objective-C runtime requirements."
            }
            Self::ObjcRuntime => {
                "Bara does not yet provide an Objective-C runtime helper boundary for the AppKit GUI fixture."
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum GuiHelloWorldActualBlockerSelectionRule {
    #[serde(rename = "first_unsupported_launch_boundary")]
    FirstUnsupportedLaunchBoundary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GuiHelloWorldActualBlockerCandidate {
    boundary: GuiHelloWorldUnsupportedLaunchBoundary,
    classification: GuiHelloWorldActualBlockerClassification,
}

impl GuiHelloWorldActualBlockerCandidate {
    const fn from_boundary(boundary: GuiHelloWorldUnsupportedLaunchBoundary) -> Self {
        Self {
            boundary,
            classification: boundary.classification(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GuiHelloWorldInitialBlockerPlan {
    unsupported_boundaries: NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries,
}

impl GuiHelloWorldInitialBlockerPlan {
    fn current() -> Self {
        Self {
            unsupported_boundaries: NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries::new(
                GuiHelloWorldUnsupportedLaunchBoundary::Loader,
                vec![
                    GuiHelloWorldUnsupportedLaunchBoundary::Import,
                    GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime,
                ],
            ),
        }
    }

    const fn selected_boundary(&self) -> GuiHelloWorldUnsupportedLaunchBoundary {
        self.unsupported_boundaries.first()
    }

    const fn selected_classification(&self) -> GuiHelloWorldActualBlockerClassification {
        self.selected_boundary().classification()
    }

    fn candidate_boundaries(&self) -> Vec<GuiHelloWorldActualBlockerCandidate> {
        self.unsupported_boundaries
            .to_vec()
            .into_iter()
            .map(GuiHelloWorldActualBlockerCandidate::from_boundary)
            .collect()
    }

    #[cfg(test)]
    fn with_first_boundary(first: GuiHelloWorldUnsupportedLaunchBoundary) -> Self {
        Self {
            unsupported_boundaries: NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries::new(
                first,
                Vec::new(),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries {
    first: GuiHelloWorldUnsupportedLaunchBoundary,
    remaining: Vec<GuiHelloWorldUnsupportedLaunchBoundary>,
}

impl NonEmptyGuiHelloWorldUnsupportedLaunchBoundaries {
    const fn new(
        first: GuiHelloWorldUnsupportedLaunchBoundary,
        remaining: Vec<GuiHelloWorldUnsupportedLaunchBoundary>,
    ) -> Self {
        Self { first, remaining }
    }

    const fn first(&self) -> GuiHelloWorldUnsupportedLaunchBoundary {
        self.first
    }

    fn to_vec(&self) -> Vec<GuiHelloWorldUnsupportedLaunchBoundary> {
        let mut boundaries = vec![self.first];
        boundaries.extend(self.remaining.iter().copied());
        boundaries
    }
}

pub(crate) fn b8_gui_hello_world_actual_launch_attempt(
    input_probe_report: &BinaryFormatProbeReport,
) -> Result<GuiHelloWorldActualLaunchBundle, X8664MachOFixtureError> {
    Ok(GuiHelloWorldActualLaunchBundle::blocked_by_initial_blocker(
        b8_gui_hello_world_case_id()?,
        GuiHelloWorldActualInputMetadata::from_report(input_probe_report),
        GuiHelloWorldInitialBlockerPlan::current(),
    ))
}

#[cfg(test)]
mod tests {
    use bara_oracle::{probe_public_binary_format, BinaryInput, CaseId, ObservedResult};

    use super::{
        b8_gui_hello_world_actual_launch_attempt, GuiHelloWorldActualBlockerClassification,
        GuiHelloWorldInitialBlockerPlan, GuiHelloWorldUnsupportedLaunchBoundary,
    };

    #[test]
    fn gui_hello_world_actual_attempt_reports_loader_blocker() {
        let probe_report = mach_o_execute_header_probe();
        let attempt = b8_gui_hello_world_actual_launch_attempt(&probe_report)
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
            "{\"schema\":\"b8_gui_hello_world_actual_launch_report_v0\",\"case_id\":\"b8_gui_hello_world\",\"actual_runtime\":\"bara_arm64_user_space\",\"status\":\"blocked\",\"input\":{\"kind\":\"mach_o_executable_image\",\"source_isa\":\"x86_64\",\"binary_format\":\"mach_o\",\"target_triple\":\"x86_64-apple-macos13\",\"gui_framework\":\"appkit\",\"probe\":{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\"},\"loader_metadata\":{\"source\":\"public_mach_o_probe\",\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}},\"sections\":{\"status\":\"modeled_from_lc_segment_64_section_table\"},\"imports\":{\"status\":\"not_modeled\"},\"relocations\":{\"status\":\"not_modeled\"}}},\"blocker\":{\"classification\":\"unsupported_loader_feature\",\"boundary\":\"loader\",\"selected_by\":\"first_unsupported_launch_boundary\",\"candidate_boundaries\":[{\"boundary\":\"loader\",\"classification\":\"unsupported_loader_feature\"},{\"boundary\":\"import\",\"classification\":\"unsupported_import\"},{\"boundary\":\"objc_runtime\",\"classification\":\"unsupported_objc_runtime_boundary\"}],\"message\":\"Bara does not yet load a complete x86_64 Mach-O GUI executable with dynamic loader, AppKit import, and Objective-C runtime requirements.\"}}"
        );
    }

    #[test]
    fn initial_blocker_plan_selects_loader_before_import_and_objc_runtime() {
        let plan = GuiHelloWorldInitialBlockerPlan::current();

        assert_eq!(
            plan.selected_classification(),
            GuiHelloWorldActualBlockerClassification::LoaderFeature
        );
        assert_eq!(
            plan.candidate_boundaries(),
            vec![
                super::GuiHelloWorldActualBlockerCandidate::from_boundary(
                    GuiHelloWorldUnsupportedLaunchBoundary::Loader
                ),
                super::GuiHelloWorldActualBlockerCandidate::from_boundary(
                    GuiHelloWorldUnsupportedLaunchBoundary::Import
                ),
                super::GuiHelloWorldActualBlockerCandidate::from_boundary(
                    GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime
                ),
            ]
        );
    }

    #[test]
    fn initial_blocker_plan_has_stable_import_and_objc_runtime_classifications() {
        let import_plan = GuiHelloWorldInitialBlockerPlan::with_first_boundary(
            GuiHelloWorldUnsupportedLaunchBoundary::Import,
        );
        let objc_runtime_plan = GuiHelloWorldInitialBlockerPlan::with_first_boundary(
            GuiHelloWorldUnsupportedLaunchBoundary::ObjcRuntime,
        );

        assert_eq!(
            import_plan.selected_classification(),
            GuiHelloWorldActualBlockerClassification::Import
        );
        assert_eq!(
            objc_runtime_plan.selected_classification(),
            GuiHelloWorldActualBlockerClassification::ObjcRuntimeBoundary
        );
    }

    fn mach_o_execute_header_probe() -> bara_oracle::BinaryFormatProbeReport {
        let input = BinaryInput::from_hex(
            "cffaedfe07000001030000000200000000000000000000000000000000000000",
        )
        .expect("minimal Mach-O executable header hex parses");
        probe_public_binary_format(&input).expect("minimal Mach-O executable header probes")
    }
}
