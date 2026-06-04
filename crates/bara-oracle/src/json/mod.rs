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
        CorpusReport, FailureKind, FailureMessage, FixtureOutcome, FixtureReport,
        MachOEntryPointCommandMetadata, MachOEntryPointFileOffset, MachOEntryPointStackSize,
        MachOFileType, MachOLoadCommandByteSize, MachOLoadCommandCount, MachOLoadCommandSummary,
        MachOLoadCommandType, MachOLoadCommands, MachOMetadata, MachOSegmentCommandHeaderMetadata,
        MachOSegmentFileOffset, MachOSegmentFileSize, MachOSegmentName, MachOSegmentVmAddr,
        ObservedResult, RecognizedMachOEntryPointCommand, RecognizedMachOSegmentCommand,
        UnsupportedMachOLoadCommand,
    };

    fn empty_load_commands() -> MachOLoadCommands {
        MachOLoadCommands::new(
            MachOLoadCommandCount::from_public_header_value(0),
            MachOLoadCommandByteSize::from_public_header_value(0),
            MachOLoadCommandSummary::empty(),
        )
    }

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
                empty_load_commands(),
            )),
        );

        assert_eq!(
            binary_format_probe_report_to_json(&report).expect("probe report serializes"),
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":0,\"byte_size\":0,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}}}}"
        );
    }

    #[test]
    fn binary_format_probe_report_serializes_unsupported_mach_o_commands_as_stable_json() {
        let report = BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
            BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                MachOFileType::Executable,
                MachOLoadCommands::new(
                    MachOLoadCommandCount::from_public_header_value(1),
                    MachOLoadCommandByteSize::from_public_header_value(8),
                    MachOLoadCommandSummary::from_unsupported_commands(vec![
                        UnsupportedMachOLoadCommand::new(
                            MachOLoadCommandType::from_public_command_value(1),
                            MachOLoadCommandByteSize::from_public_header_value(8),
                        ),
                    ]),
                ),
            )),
        );

        assert_eq!(
            binary_format_probe_report_to_json(&report).expect("probe report serializes"),
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":1,\"byte_size\":8,\"recognized_entry_points\":[],\"recognized_segments\":[],\"unsupported_commands\":[{\"command\":1,\"byte_size\":8}]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}}}}"
        );
    }

    #[test]
    fn binary_format_probe_report_serializes_recognized_mach_o_entry_points_as_stable_json() {
        let report = BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
            BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                MachOFileType::Executable,
                MachOLoadCommands::new(
                    MachOLoadCommandCount::from_public_header_value(1),
                    MachOLoadCommandByteSize::from_public_header_value(24),
                    MachOLoadCommandSummary::new(
                        vec![RecognizedMachOEntryPointCommand::new(
                            MachOLoadCommandByteSize::from_public_header_value(24),
                            MachOEntryPointCommandMetadata::new(
                                MachOEntryPointFileOffset::from_public_entry_point_value(0x1234),
                                MachOEntryPointStackSize::from_public_entry_point_value(0x2000),
                            ),
                        )],
                        Vec::<RecognizedMachOSegmentCommand>::new(),
                        Vec::<UnsupportedMachOLoadCommand>::new(),
                    ),
                ),
            )),
        );

        assert_eq!(
            binary_format_probe_report_to_json(&report).expect("probe report serializes"),
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":1,\"byte_size\":24,\"recognized_entry_points\":[{\"byte_size\":24,\"entryoff\":4660,\"stacksize\":8192}],\"recognized_segments\":[],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_segment\"}}}}"
        );
    }

    #[test]
    fn binary_format_probe_report_serializes_mach_o_entry_point_and_segment_as_stable_json() {
        let report = BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
            BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                MachOFileType::Executable,
                MachOLoadCommands::new(
                    MachOLoadCommandCount::from_public_header_value(2),
                    MachOLoadCommandByteSize::from_public_header_value(96),
                    MachOLoadCommandSummary::new(
                        vec![RecognizedMachOEntryPointCommand::new(
                            MachOLoadCommandByteSize::from_public_header_value(24),
                            MachOEntryPointCommandMetadata::new(
                                MachOEntryPointFileOffset::from_public_entry_point_value(0x1234),
                                MachOEntryPointStackSize::from_public_entry_point_value(0x2000),
                            ),
                        )],
                        vec![RecognizedMachOSegmentCommand::new(
                            MachOLoadCommandByteSize::from_public_header_value(72),
                            MachOSegmentCommandHeaderMetadata::new(
                                MachOSegmentName::from_public_fixed_field(
                                    b"__TEXT\0\0\0\0\0\0\0\0\0\0",
                                )
                                .expect("test segment name is valid"),
                                MachOSegmentVmAddr::from_public_segment_value(0x1_0000_0000),
                                MachOSegmentFileOffset::from_public_segment_value(0),
                                MachOSegmentFileSize::from_public_segment_value(0x1234),
                            ),
                        )],
                        Vec::<UnsupportedMachOLoadCommand>::new(),
                    ),
                ),
            )),
        );

        assert_eq!(
            binary_format_probe_report_to_json(&report).expect("probe report serializes"),
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":2,\"byte_size\":96,\"recognized_entry_points\":[{\"byte_size\":24,\"entryoff\":4660,\"stacksize\":8192}],\"recognized_segments\":[{\"byte_size\":72,\"name\":\"__TEXT\",\"vmaddr\":4294967296,\"fileoff\":0,\"filesize\":4660}],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"entry_point_outside_segment\"}}}}"
        );
    }

    #[test]
    fn binary_format_probe_report_serializes_mach_o_entry_point_inside_segment_as_stable_json() {
        let report = BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
            BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                MachOFileType::Executable,
                MachOLoadCommands::new(
                    MachOLoadCommandCount::from_public_header_value(2),
                    MachOLoadCommandByteSize::from_public_header_value(96),
                    MachOLoadCommandSummary::new(
                        vec![RecognizedMachOEntryPointCommand::new(
                            MachOLoadCommandByteSize::from_public_header_value(24),
                            MachOEntryPointCommandMetadata::new(
                                MachOEntryPointFileOffset::from_public_entry_point_value(0x1234),
                                MachOEntryPointStackSize::from_public_entry_point_value(0x2000),
                            ),
                        )],
                        vec![RecognizedMachOSegmentCommand::new(
                            MachOLoadCommandByteSize::from_public_header_value(72),
                            MachOSegmentCommandHeaderMetadata::new(
                                MachOSegmentName::from_public_fixed_field(
                                    b"__TEXT\0\0\0\0\0\0\0\0\0\0",
                                )
                                .expect("test segment name is valid"),
                                MachOSegmentVmAddr::from_public_segment_value(0x1_0000_0000),
                                MachOSegmentFileOffset::from_public_segment_value(0),
                                MachOSegmentFileSize::from_public_segment_value(0x1235),
                            ),
                        )],
                        Vec::<UnsupportedMachOLoadCommand>::new(),
                    ),
                ),
            )),
        );

        assert_eq!(
            binary_format_probe_report_to_json(&report).expect("probe report serializes"),
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":2,\"byte_size\":96,\"recognized_entry_points\":[{\"byte_size\":24,\"entryoff\":4660,\"stacksize\":8192}],\"recognized_segments\":[{\"byte_size\":72,\"name\":\"__TEXT\",\"vmaddr\":4294967296,\"fileoff\":0,\"filesize\":4661}],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"unsupported_image_mapping\"}}}}"
        );
    }

    #[test]
    fn binary_format_probe_report_serializes_recognized_mach_o_segments_as_stable_json() {
        let report = BinaryFormatProbeReport::new(
            BinaryFormat::MachO64LittleEndian,
            BinaryFormatProbeStatus::RecognizedButUnsupported,
            BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                MachOFileType::Executable,
                MachOLoadCommands::new(
                    MachOLoadCommandCount::from_public_header_value(1),
                    MachOLoadCommandByteSize::from_public_header_value(72),
                    MachOLoadCommandSummary::new(
                        Vec::<RecognizedMachOEntryPointCommand>::new(),
                        vec![RecognizedMachOSegmentCommand::new(
                            MachOLoadCommandByteSize::from_public_header_value(72),
                            MachOSegmentCommandHeaderMetadata::new(
                                MachOSegmentName::from_public_fixed_field(
                                    b"__TEXT\0\0\0\0\0\0\0\0\0\0",
                                )
                                .expect("test segment name is valid"),
                                MachOSegmentVmAddr::from_public_segment_value(0x1_0000_0000),
                                MachOSegmentFileOffset::from_public_segment_value(0),
                                MachOSegmentFileSize::from_public_segment_value(0x1234),
                            ),
                        )],
                        Vec::<UnsupportedMachOLoadCommand>::new(),
                    ),
                ),
            )),
        );

        assert_eq!(
            binary_format_probe_report_to_json(&report).expect("probe report serializes"),
            "{\"format\":\"mach_o_64_little_endian\",\"status\":\"recognized_but_unsupported\",\"metadata\":{\"mach_o\":{\"file_type\":\"executable\",\"load_commands\":{\"count\":1,\"byte_size\":72,\"recognized_entry_points\":[],\"recognized_segments\":[{\"byte_size\":72,\"name\":\"__TEXT\",\"vmaddr\":4294967296,\"fileoff\":0,\"filesize\":4660}],\"unsupported_commands\":[]},\"executable_image_conversion\":{\"status\":\"not_convertible\",\"blocker\":\"missing_entry_point\"}}}}"
        );
    }

    #[test]
    fn binary_format_probe_report_parses_from_expected_json() {
        let report = binary_format_probe_report_from_json(
            "{\n  \"format\": \"mach_o_64_little_endian\",\n  \"status\": \"recognized_but_unsupported\",\n  \"metadata\": {\n    \"mach_o\": {\n      \"file_type\": \"executable\",\n      \"load_commands\": {\n        \"count\": 0,\n        \"byte_size\": 0,\n        \"recognized_entry_points\": [],\n        \"recognized_segments\": [],\n        \"unsupported_commands\": []\n      },\n      \"executable_image_conversion\": {\n        \"status\": \"not_convertible\",\n        \"blocker\": \"missing_entry_point\"\n      }\n    }\n  }\n}\n",
        )
        .expect("probe report json parses");

        assert_eq!(
            report,
            BinaryFormatProbeReport::new(
                BinaryFormat::MachO64LittleEndian,
                BinaryFormatProbeStatus::RecognizedButUnsupported,
                BinaryFormatProbeMetadata::mach_o(MachOMetadata::new(
                    MachOFileType::Executable,
                    empty_load_commands()
                ))
            )
        );
    }
}
