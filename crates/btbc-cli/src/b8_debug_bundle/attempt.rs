use bara_arm64::{
    emit_program, EmitError, TranslationArtifact, TranslationCacheIdentity,
    TranslationSourceIdentity, TranslationTarget, TranslatorVersion,
};
use bara_ir::{Program, Terminator, UnsupportedReason};
use bara_isa_x86::{decode_function, lift_decoded_function_with_image_metadata};
use bara_oracle::TestCase;
use bara_runtime::{
    dispatch_entry_once, dispatch_entry_without_artifact, GuestRegisterState, GuestRuntimePhase,
    GuestRuntimeState, GuestStackState, MachOExecutableImagePreparation,
};

use super::report::{
    B8DebugArtifactReport, B8DebugBlockerReport, B8DebugDecodeReport, B8DebugLaunchReport,
    B8DebugProcessedPcRange, B8DebugRuntimeAttemptReport, B8DebugRuntimeRunScope,
    B8DebugUnsupportedInstructionReport,
};
use super::translation_artifact::B8DebugTranslationArtifactReport;

use crate::function_run::{
    FunctionArtifactReport, FunctionCompiledIrArtifact, FunctionFixupsArtifact,
    FunctionHelpersArtifact, FunctionPcMapArtifact, FunctionRunError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct B8RealEntryAttempt {
    pub(super) decode_report: B8DebugDecodeReport,
    pub(super) lift_ir: B8DebugArtifactReport<FunctionCompiledIrArtifact>,
    pub(super) emit_report: B8DebugArtifactReport<FunctionArtifactReport>,
    pub(super) translation_artifact: B8DebugArtifactReport<B8DebugTranslationArtifactReport>,
    pub(super) pcmap: B8DebugArtifactReport<FunctionPcMapArtifact>,
    pub(super) fixups: B8DebugArtifactReport<FunctionFixupsArtifact>,
    pub(super) helpers: B8DebugArtifactReport<FunctionHelpersArtifact>,
    pub(super) runtime_report: B8DebugRuntimeAttemptReport,
    pub(super) launch_report: B8DebugLaunchReport,
    pub(super) blocker_report: B8DebugBlockerReport,
}

impl B8RealEntryAttempt {
    pub(super) fn run(
        test_case: &TestCase,
        image_preparation: &MachOExecutableImagePreparation,
        source_identity: TranslationSourceIdentity,
    ) -> Self {
        let image_metadata = image_preparation.program_image_metadata();
        let decoded_result = decode_function(test_case.x86_bytes());
        let decode_report = B8DebugDecodeReport::from_result(decoded_result.as_ref());
        let decoded = match decoded_result {
            Ok(decoded) => decoded,
            Err(error) => {
                let blocker_report = B8DebugBlockerReport::from_decode_error(&error);
                return Self::blocked_without_ir(
                    test_case,
                    decode_report,
                    None,
                    "decode failed",
                    blocker_report,
                );
            }
        };

        let processed_pc_range = Some(B8DebugProcessedPcRange::from_decoded(&decoded));
        let first_unsupported_instruction =
            B8DebugUnsupportedInstructionReport::from_decoded(&decoded);
        let program = match lift_decoded_function_with_image_metadata(&decoded, image_metadata) {
            Ok(program) => program,
            Err(error) => {
                let blocker_report = first_unsupported_instruction
                    .as_ref()
                    .map(B8DebugBlockerReport::from_unsupported_instruction)
                    .unwrap_or_else(|| B8DebugBlockerReport::from_lift_error(&error));
                return Self::blocked_without_ir(
                    test_case,
                    decode_report,
                    processed_pc_range,
                    format!("lift failed: {error:?}"),
                    blocker_report,
                );
            }
        };

        let lift_ir =
            B8DebugArtifactReport::available(FunctionCompiledIrArtifact::from_program(&program));
        if let Some(reason) = frontier_unsupported_terminator_reason(&program) {
            let run_error = FunctionRunError::Emit(EmitError::UnsupportedIr {
                reason: reason.clone(),
            });
            let blocker_report = B8DebugBlockerReport::from_function_error(&run_error);
            let dispatch_outcome = dispatch_entry_without_artifact(
                image_preparation,
                entry_initial_runtime_state(image_preparation),
            );
            return Self {
                decode_report,
                lift_ir,
                emit_report: B8DebugArtifactReport::failed(run_error.to_string()),
                translation_artifact: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                pcmap: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                fixups: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                helpers: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                runtime_report: B8DebugRuntimeAttemptReport::from_dispatch_outcome(
                    &dispatch_outcome,
                    B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
                ),
                launch_report: B8DebugLaunchReport::from_attempt(
                    test_case,
                    processed_pc_range,
                    &blocker_report,
                ),
                blocker_report,
            };
        }
        let emitted = match emit_program(&program) {
            Ok(emitted) => emitted,
            Err(error) => {
                let run_error = FunctionRunError::Emit(error);
                let blocker_report = first_unsupported_instruction
                    .as_ref()
                    .map(B8DebugBlockerReport::from_unsupported_instruction)
                    .unwrap_or_else(|| B8DebugBlockerReport::from_function_error(&run_error));
                return Self {
                    decode_report,
                    lift_ir,
                    emit_report: B8DebugArtifactReport::failed(run_error.to_string()),
                    translation_artifact: B8DebugArtifactReport::skipped("emit failed"),
                    pcmap: B8DebugArtifactReport::skipped("emit failed"),
                    fixups: B8DebugArtifactReport::skipped("emit failed"),
                    helpers: B8DebugArtifactReport::skipped("emit failed"),
                    runtime_report: B8DebugRuntimeAttemptReport::skipped(
                        "emit failed",
                        B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
                    ),
                    launch_report: B8DebugLaunchReport::from_attempt(
                        test_case,
                        processed_pc_range,
                        &blocker_report,
                    ),
                    blocker_report,
                };
            }
        };

        let cache_identity = TranslationCacheIdentity::new(
            source_identity.source_hash(),
            TranslatorVersion::current(),
            TranslationTarget::Arm64MacOs,
        );
        let artifact = match TranslationArtifact::new(source_identity, emitted, cache_identity) {
            Ok(artifact) => artifact,
            Err(error) => {
                let run_error = FunctionRunError::TranslationArtifact(error);
                let blocker_report = B8DebugBlockerReport::from_function_error(&run_error);
                return Self {
                    decode_report,
                    lift_ir,
                    emit_report: B8DebugArtifactReport::failed(run_error.to_string()),
                    translation_artifact: B8DebugArtifactReport::failed(run_error.to_string()),
                    pcmap: B8DebugArtifactReport::skipped(
                        "translation artifact construction failed",
                    ),
                    fixups: B8DebugArtifactReport::skipped(
                        "translation artifact construction failed",
                    ),
                    helpers: B8DebugArtifactReport::skipped(
                        "translation artifact construction failed",
                    ),
                    runtime_report: B8DebugRuntimeAttemptReport::failed(
                        &run_error,
                        B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
                    ),
                    launch_report: B8DebugLaunchReport::from_attempt(
                        test_case,
                        processed_pc_range,
                        &blocker_report,
                    ),
                    blocker_report,
                };
            }
        };
        let emitted = artifact.emitted_function();
        let emit_report = B8DebugArtifactReport::available(
            FunctionArtifactReport::from_source_and_emitted(test_case, emitted),
        );
        let translation_artifact_report =
            B8DebugTranslationArtifactReport::from_artifact(&artifact);
        let pcmap = B8DebugArtifactReport::available(translation_artifact_report.pc_map().clone());
        let fixups = B8DebugArtifactReport::available(translation_artifact_report.fixups().clone());
        let helpers = B8DebugArtifactReport::available(
            translation_artifact_report.helper_requirements().clone(),
        );
        let translation_artifact = B8DebugArtifactReport::available(translation_artifact_report);
        let initial_state = entry_initial_runtime_state(image_preparation);
        let dispatch_outcome = dispatch_entry_once(image_preparation, &artifact, initial_state);
        let blocker_report = B8DebugBlockerReport::from_dispatch_outcome(&dispatch_outcome);
        Self {
            decode_report,
            lift_ir,
            emit_report,
            translation_artifact,
            pcmap,
            fixups,
            helpers,
            runtime_report: B8DebugRuntimeAttemptReport::from_dispatch_outcome(
                &dispatch_outcome,
                B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
            ),
            launch_report: B8DebugLaunchReport::from_attempt(
                test_case,
                processed_pc_range,
                &blocker_report,
            ),
            blocker_report,
        }
    }

    fn blocked_without_ir(
        test_case: &TestCase,
        decode_report: B8DebugDecodeReport,
        processed_pc_range: Option<B8DebugProcessedPcRange>,
        reason: impl Into<String>,
        blocker_report: B8DebugBlockerReport,
    ) -> Self {
        let reason = reason.into();
        Self {
            decode_report,
            lift_ir: B8DebugArtifactReport::failed(reason.clone()),
            emit_report: B8DebugArtifactReport::skipped(reason.clone()),
            translation_artifact: B8DebugArtifactReport::skipped(reason.clone()),
            pcmap: B8DebugArtifactReport::skipped(reason.clone()),
            fixups: B8DebugArtifactReport::skipped(reason.clone()),
            helpers: B8DebugArtifactReport::skipped(reason.clone()),
            runtime_report: B8DebugRuntimeAttemptReport::skipped(
                reason,
                B8DebugRuntimeRunScope::RealLcMainEntryFirstBlock,
            ),
            launch_report: B8DebugLaunchReport::from_attempt(
                test_case,
                processed_pc_range,
                &blocker_report,
            ),
            blocker_report,
        }
    }
}

fn entry_initial_runtime_state(
    image_preparation: &MachOExecutableImagePreparation,
) -> GuestRuntimeState {
    GuestRuntimeState::new(
        image_preparation.initial_program_counter(),
        GuestRegisterState::empty(),
        GuestStackState::unmaterialized(),
        GuestRuntimePhase::Ready,
    )
    .expect("entry preparation and explicit unmaterialized stack form a valid ready state")
}

fn frontier_unsupported_terminator_reason(program: &Program) -> Option<&UnsupportedReason> {
    program
        .blocks()
        .iter()
        .rev()
        .find_map(|block| match block.terminator() {
            Terminator::Unsupported { reason } => Some(reason),
            Terminator::Return
            | Terminator::BoundaryRequest { .. }
            | Terminator::Fallthrough { .. }
            | Terminator::DirectJump { .. }
            | Terminator::DirectCall { .. }
            | Terminator::CondJump { .. } => None,
        })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bara_arm64::{TranslationSourceHash, TranslationSourceIdentity};
    use bara_ir::{
        ProgramImageMappedByteSegment, ProgramImageMappedBytes, ProgramImageMetadata,
        ProgramImageRange, X86Va,
    };
    use bara_oracle::test_case_from_json;
    use bara_runtime::{
        GuestImageMetadata, MachOExecutableCodeRange, MachOExecutableEntryPoint,
        MachOExecutableImagePreparation, MachOImage,
    };

    use super::B8RealEntryAttempt;

    #[test]
    fn return_42_reaches_the_typed_translation_artifact_runtime_branch() {
        let test_case = test_case_from_json(include_str!("../../../../tests/cases/return_42.json"))
            .expect("return_42 testcase parses");
        let source_hash = TranslationSourceHash::from_str(
            "0000000000000000000000000000000000000000000000000000000000000001",
        )
        .expect("test source hash is valid");

        let attempt = B8RealEntryAttempt::run(
            &test_case,
            &image_preparation(),
            TranslationSourceIdentity::new(source_hash),
        );
        let runtime_report =
            serde_json::to_value(&attempt.runtime_report).expect("runtime report serializes");
        let translation_artifact = serde_json::to_value(&attempt.translation_artifact)
            .expect("translation artifact report serializes");

        assert_eq!(runtime_report["schema"], "b8_debug_runtime_attempt_v1");
        assert_eq!(
            runtime_report["run_scope"],
            "real_lc_main_entry_first_block"
        );
        assert_eq!(
            runtime_report["dispatch"]["schema"],
            "b8_debug_entry_dispatch_v0"
        );
        assert_eq!(
            runtime_report["dispatch"]["initial_state"]["program_counter"],
            0
        );
        assert_eq!(
            runtime_report["dispatch"]["initial_state"]["stack"],
            "unmaterialized"
        );
        assert_eq!(translation_artifact["status"], "available");
        assert_eq!(
            translation_artifact["value"]["schema"],
            "b8_debug_translation_artifact_v0"
        );
        assert_eq!(
            translation_artifact["value"]["source_identity"]["source_hash"],
            "0000000000000000000000000000000000000000000000000000000000000001"
        );
        assert_eq!(
            translation_artifact["value"]["cache_identity"]["source_hash"],
            translation_artifact["value"]["source_identity"]["source_hash"]
        );
        assert_eq!(
            translation_artifact["value"]["pc_map"],
            serde_json::to_value(&attempt.pcmap).expect("pcmap report serializes")["value"]
        );
        assert_eq!(
            translation_artifact["value"]["fixups"],
            serde_json::to_value(&attempt.fixups).expect("fixups report serializes")["value"]
        );
        assert_eq!(
            translation_artifact["value"]["helper_requirements"],
            serde_json::to_value(&attempt.helpers).expect("helpers report serializes")["value"]
        );
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            assert_eq!(runtime_report["status"], "executed");
            assert_eq!(runtime_report["return_value"], 42);
            assert_eq!(runtime_report["stdout"], "");
            assert!(runtime_report["error"].is_null());
            assert_eq!(runtime_report["dispatch"]["outcome"], "return");
            assert_eq!(runtime_report["dispatch"]["final_state"]["rax"], 42);
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            assert_eq!(runtime_report["status"], "failed");
            assert!(runtime_report["return_value"].is_null());
            assert!(runtime_report["stdout"].is_null());
            assert!(runtime_report["error"]
                .as_str()
                .is_some_and(|error| error.contains("unsupported host")));
        }
    }

    fn image_preparation() -> MachOExecutableImagePreparation {
        let range =
            ProgramImageRange::new(X86Va::new(0), X86Va::new(6)).expect("test range is valid");
        let mapped = ProgramImageMappedByteSegment::new(range, vec![0xb8, 42, 0, 0, 0, 0xc3])
            .expect("mapped bytes cover the range");
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            Default::default(),
            ProgramImageMappedBytes::from_segments([mapped]),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        let image = MachOImage::executable_from_code_range(
            MachOExecutableEntryPoint::new(range.start()),
            MachOExecutableCodeRange::new(range),
            GuestImageMetadata::from_program_image_metadata(&metadata),
        )
        .expect("test image is valid");
        MachOExecutableImagePreparation::try_from_snapshot(image.executable_snapshot())
            .expect("test image preparation is valid")
    }
}
