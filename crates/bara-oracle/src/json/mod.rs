use crate::{BinaryFormatProbeReport, CorpusReport, ObservedResult};

#[derive(Debug)]
pub struct JsonError {
    source: serde_json::Error,
}

impl JsonError {
    pub const fn new(source: serde_json::Error) -> Self {
        Self { source }
    }

    pub const fn source(&self) -> &serde_json::Error {
        &self.source
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "json error: {}", self.source)
    }
}

impl std::error::Error for JsonError {}

pub fn observed_result_to_json(result: &ObservedResult) -> Result<String, JsonError> {
    serde_json::to_string(result).map_err(JsonError::new)
}

pub fn observed_result_from_json(input: &str) -> Result<ObservedResult, JsonError> {
    serde_json::from_str(input).map_err(JsonError::new)
}

pub fn corpus_report_to_json(report: &CorpusReport) -> Result<String, JsonError> {
    serde_json::to_string(report).map_err(JsonError::new)
}

pub fn binary_format_probe_report_to_json(
    report: &BinaryFormatProbeReport,
) -> Result<String, JsonError> {
    serde_json::to_string(report).map_err(JsonError::new)
}

pub fn binary_format_probe_report_from_json(
    input: &str,
) -> Result<BinaryFormatProbeReport, JsonError> {
    serde_json::from_str(input).map_err(JsonError::new)
}

#[cfg(test)]
mod tests {
    use crate::{
        binary_format_probe_report_from_json, binary_format_probe_report_to_json,
        corpus_report_to_json, observed_result_from_json, observed_result_to_json, BinaryFormat,
        BinaryFormatProbeMetadata, BinaryFormatProbeReport, BinaryFormatProbeStatus, CaseId,
        CorpusReport, FailureKind, FailureMessage, FixtureOutcome, FixtureReport, MachOFileType,
        MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommands, MachOMetadata,
        ObservedResult,
    };

    const EMPTY_LOAD_COMMANDS: MachOLoadCommands = MachOLoadCommands::new(
        MachOLoadCommandCount::from_public_header_value(0),
        MachOLoadCommandByteSize::from_public_header_value(0),
    );

    #[test]
    fn observed_result_serializes_as_m1_json() {
        let result = ObservedResult::new(
            CaseId::new("return_42").expect("test case id is non-empty"),
            0,
            42,
            String::new(),
            String::new(),
        );

        assert_eq!(
            observed_result_to_json(&result).expect("test result serializes"),
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}"
        );
    }

    #[test]
    fn observed_result_parses_from_m1_json() {
        let result = observed_result_from_json(
            "{\"case_id\":\"return_42\",\"exit_status\":0,\"return_value\":42,\"stdout\":\"\",\"stderr\":\"\"}",
        )
        .expect("expected result json parses");

        assert_eq!(
            result,
            ObservedResult::new(
                CaseId::new("return_42").expect("test case id is non-empty"),
                0,
                42,
                String::new(),
                String::new()
            )
        );
    }

    #[test]
    fn corpus_report_serializes_as_stable_json() {
        let report = vec![
            FixtureReport::new(
                CaseId::new("return_42").expect("case id is non-empty"),
                FixtureOutcome::Passed,
            ),
            FixtureReport::new(
                CaseId::new("bad_case").expect("case id is non-empty"),
                FixtureOutcome::failed(
                    FailureKind::DecodeError,
                    FailureMessage::from("decode failed"),
                ),
            ),
        ]
        .into_iter()
        .collect::<CorpusReport>();

        assert_eq!(
            corpus_report_to_json(&report).expect("report serializes"),
            "{\"fixtures\":[{\"case_id\":\"return_42\",\"outcome\":\"passed\"},{\"case_id\":\"bad_case\",\"outcome\":{\"failed\":{\"kind\":\"decode_error\",\"message\":\"decode failed\"}}}]}"
        );
    }

    #[test]
    fn binary_format_probe_report_serializes_as_stable_json() {
        let report = BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
            BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                MachOFileType::Executable,
                EMPTY_LOAD_COMMANDS,
            )),
        );

        assert_eq!(
            binary_format_probe_report_to_json(&report).expect("probe report serializes"),
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0}}}}"
        );
    }

    #[test]
    fn binary_format_probe_report_parses_from_expected_json() {
        let report = binary_format_probe_report_from_json(
            "{\n  \"format\": \"mach_o_64_little_endian\",\n  \"status\": \"recognized_but_unsupported\",\n  \"metadata\": {\n    \"mach_o\": {\n      \"file_type\": \"executable\",\n      \"load_commands\": {\n        \"count\": 0,\n        \"byte_size\": 0\n      }\n    }\n  }\n}\n",
        )
        .expect("probe report json parses");

        assert_eq!(
            report,
            BinaryFormatProbeReport::new(
                BinaryFormat::MachO64LittleEndian,
                BinaryFormatProbeStatus::RecognizedButUnsupported,
                BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                    MachOFileType::Executable,
                    EMPTY_LOAD_COMMANDS
                ))
            )
        );
    }
}
