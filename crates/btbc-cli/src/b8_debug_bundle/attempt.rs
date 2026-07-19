use bara_arm64::{
    emit_program, EmitError, TranslationArtifact, TranslationCacheIdentity,
    TranslationSourceIdentity, TranslationTarget, TranslatorVersion,
};
use bara_ir::{Program, ProgramImageMetadata, Terminator, UnsupportedReason};
use bara_isa_x86::{decode_function, lift_decoded_function_with_image_metadata};
use bara_oracle::TestCase;

use super::report::{
    B8DebugArtifactReport, B8DebugBlockerReport, B8DebugDecodeReport, B8DebugLaunchReport,
    B8DebugProcessedPcRange, B8DebugRuntimeAttemptReport, B8DebugRuntimeRunScope,
    B8DebugUnsupportedInstructionReport,
};

use crate::function_run::{
    run_test_case_translation_artifact, FunctionArtifactReport, FunctionCompiledIrArtifact,
    FunctionFixupsArtifact, FunctionHelpersArtifact, FunctionPcMapArtifact, FunctionRunError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct B8RealEntryAttempt {
    pub(super) decode_report: B8DebugDecodeReport,
    pub(super) lift_ir: B8DebugArtifactReport<FunctionCompiledIrArtifact>,
    pub(super) emit_report: B8DebugArtifactReport<FunctionArtifactReport>,
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
        image_metadata: &ProgramImageMetadata,
        source_identity: TranslationSourceIdentity,
    ) -> Self {
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
        let program =
            match lift_decoded_function_with_image_metadata(&decoded, image_metadata.clone()) {
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
            return Self {
                decode_report,
                lift_ir,
                emit_report: B8DebugArtifactReport::failed(run_error.to_string()),
                pcmap: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                fixups: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                helpers: B8DebugArtifactReport::skipped("unsupported IR terminator"),
                runtime_report: B8DebugRuntimeAttemptReport::skipped(
                    "unsupported IR terminator",
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
        let pcmap =
            B8DebugArtifactReport::available(FunctionPcMapArtifact::from_entries(emitted.pc_map()));
        let fixups = B8DebugArtifactReport::available(FunctionFixupsArtifact::from_fixups(
            emitted.branch_fixups(),
        ));
        let helpers = B8DebugArtifactReport::available(FunctionHelpersArtifact::from_requests(
            emitted.host_trap_requests(),
        ));
        match run_test_case_translation_artifact(test_case, &artifact) {
            Ok(result) => {
                let blocker_report = B8DebugBlockerReport::none();
                Self {
                    decode_report,
                    lift_ir,
                    emit_report,
                    pcmap,
                    fixups,
                    helpers,
                    runtime_report: B8DebugRuntimeAttemptReport::from_result(
                        &result,
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
            Err(error) => {
                let blocker_report = B8DebugBlockerReport::from_function_error(&error);
                Self {
                    decode_report,
                    lift_ir,
                    emit_report,
                    pcmap,
                    fixups,
                    helpers,
                    runtime_report: B8DebugRuntimeAttemptReport::failed(
                        &error,
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
    use bara_ir::ProgramImageMetadata;
    use bara_oracle::test_case_from_json;

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
            &ProgramImageMetadata::empty(),
            TranslationSourceIdentity::new(source_hash),
        );
        let runtime_report =
            serde_json::to_value(&attempt.runtime_report).expect("runtime report serializes");

        assert_eq!(runtime_report["schema"], "b8_debug_runtime_attempt_v0");
        assert_eq!(
            runtime_report["run_scope"],
            "real_lc_main_entry_first_block"
        );
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            assert_eq!(runtime_report["status"], "executed");
            assert_eq!(runtime_report["return_value"], 42);
            assert_eq!(runtime_report["stdout"], "");
            assert!(runtime_report["error"].is_null());
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
}
