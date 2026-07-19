use bara_arm64::{TranslationArtifact, TranslationTarget};
use serde::Serialize;

use super::encode_lower_hex;
use crate::function_run::{FunctionFixupsArtifact, FunctionHelpersArtifact, FunctionPcMapArtifact};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct B8DebugTranslationArtifactReport {
    schema: &'static str,
    arm64_code: B8DebugArm64CodeReport,
    pc_map: FunctionPcMapArtifact,
    fixups: FunctionFixupsArtifact,
    helper_requirements: FunctionHelpersArtifact,
    source_identity: B8DebugTranslationSourceIdentityReport,
    cache_identity: B8DebugTranslationCacheIdentityReport,
}

impl B8DebugTranslationArtifactReport {
    pub(super) fn from_artifact(artifact: &TranslationArtifact) -> Self {
        let emitted = artifact.emitted_function();
        let cache_identity = artifact.cache_identity();

        Self {
            schema: "b8_debug_translation_artifact_v0",
            arm64_code: B8DebugArm64CodeReport {
                bytes_hex: encode_lower_hex(emitted.code().bytes()),
            },
            pc_map: FunctionPcMapArtifact::from_entries(emitted.pc_map()),
            fixups: FunctionFixupsArtifact::from_fixups(emitted.branch_fixups()),
            helper_requirements: FunctionHelpersArtifact::from_requests(
                emitted.host_trap_requests(),
            ),
            source_identity: B8DebugTranslationSourceIdentityReport {
                source_hash: artifact.source_identity().source_hash().to_string(),
            },
            cache_identity: B8DebugTranslationCacheIdentityReport {
                source_hash: cache_identity.source_hash().to_string(),
                translator_version: cache_identity.translator_version().to_string(),
                target: B8DebugTranslationTarget::from_target(cache_identity.target()),
            },
        }
    }

    pub(super) const fn pc_map(&self) -> &FunctionPcMapArtifact {
        &self.pc_map
    }

    pub(super) const fn fixups(&self) -> &FunctionFixupsArtifact {
        &self.fixups
    }

    pub(super) const fn helper_requirements(&self) -> &FunctionHelpersArtifact {
        &self.helper_requirements
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugArm64CodeReport {
    bytes_hex: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugTranslationSourceIdentityReport {
    source_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct B8DebugTranslationCacheIdentityReport {
    source_hash: String,
    translator_version: String,
    target: B8DebugTranslationTarget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum B8DebugTranslationTarget {
    #[serde(rename = "arm64_macos")]
    Arm64MacOs,
}

impl B8DebugTranslationTarget {
    const fn from_target(target: TranslationTarget) -> Self {
        match target {
            TranslationTarget::Arm64MacOs => Self::Arm64MacOs,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bara_arm64::{
        emit_program, TranslationArtifact, TranslationCacheIdentity, TranslationSourceHash,
        TranslationSourceIdentity, TranslationTarget, TranslatorVersion,
    };
    use bara_isa_x86::{decode_function, lift_decoded_function};
    use bara_oracle::{test_case_from_json, TestCase};

    use super::B8DebugTranslationArtifactReport;

    #[test]
    fn report_serializes_the_exact_artifact_code_metadata_and_identities() {
        let branch_case = test_case_from_json(include_str!(
            "../../../../tests/cases/branch_eq_return_42.json"
        ))
        .expect("branch fixture parses");
        let artifact = translation_artifact(&branch_case);

        let report = B8DebugTranslationArtifactReport::from_artifact(&artifact);
        let json = serde_json::to_value(report).expect("artifact report serializes");

        assert_eq!(json["schema"], "b8_debug_translation_artifact_v0");
        assert_eq!(
            json["arm64_code"]["bytes_hex"],
            crate::b8_debug_bundle::encode_lower_hex(artifact.emitted_function().code().bytes())
        );
        assert!(json["pc_map"]["entries"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty()));
        assert!(json["fixups"]["fixups"]
            .as_array()
            .is_some_and(|fixups| !fixups.is_empty()));
        assert_eq!(
            json["source_identity"],
            serde_json::json!({
                "source_hash": "000000000000000000000000000000000000000000000000000000000000002a"
            })
        );
        assert_eq!(
            json["cache_identity"],
            serde_json::json!({
                "source_hash": "000000000000000000000000000000000000000000000000000000000000002a",
                "translator_version": TranslatorVersion::current().to_string(),
                "target": "arm64_macos"
            })
        );
    }

    #[test]
    fn report_serializes_non_empty_helper_requirements_from_the_artifact() {
        let stdout_case = test_case_from_json(include_str!(
            "../../../../tests/cases/stdout_trap_return_0.json"
        ))
        .expect("stdout fixture parses");
        let artifact = translation_artifact(&stdout_case);

        let report = B8DebugTranslationArtifactReport::from_artifact(&artifact);
        let json = serde_json::to_value(report).expect("artifact report serializes");

        assert_eq!(
            json["helper_requirements"]["helpers"],
            serde_json::json!([{ "kind": "write_stdout" }])
        );
    }

    fn translation_artifact(test_case: &TestCase) -> TranslationArtifact {
        let decoded = decode_function(test_case.x86_bytes()).expect("fixture decodes");
        let program = lift_decoded_function(&decoded).expect("fixture lifts");
        let emitted = emit_program(&program).expect("fixture emits");
        let source_hash = TranslationSourceHash::from_str(
            "000000000000000000000000000000000000000000000000000000000000002a",
        )
        .expect("test source hash parses");
        let source_identity = TranslationSourceIdentity::new(source_hash);
        let cache_identity = TranslationCacheIdentity::new(
            source_hash,
            TranslatorVersion::current(),
            TranslationTarget::Arm64MacOs,
        );

        TranslationArtifact::new(source_identity, emitted, cache_identity)
            .expect("translation artifact is valid")
    }
}
