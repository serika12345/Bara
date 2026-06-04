use super::mach_o_load_command::{
    MachOLoadCommandSummary, RecognizedMachOEntryPointCommand, RecognizedMachOSegmentCommand,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOExecutableImageConversion(MachOExecutableImageConversionState);

impl MachOExecutableImageConversion {
    pub const fn not_convertible(blocker: MachOExecutableImageConversionBlocker) -> Self {
        Self(MachOExecutableImageConversionState::NotConvertible { blocker })
    }

    pub fn convertible(
        entry_point: RecognizedMachOEntryPointCommand,
        segment: RecognizedMachOSegmentCommand,
    ) -> Self {
        Self(MachOExecutableImageConversionState::Convertible {
            entry_point,
            segment,
        })
    }

    pub const fn status(&self) -> MachOExecutableImageConversionStatus {
        match self.0 {
            MachOExecutableImageConversionState::Convertible { .. } => {
                MachOExecutableImageConversionStatus::Convertible
            }
            MachOExecutableImageConversionState::NotConvertible { .. } => {
                MachOExecutableImageConversionStatus::NotConvertible
            }
        }
    }

    pub const fn blocker(&self) -> Option<MachOExecutableImageConversionBlocker> {
        match self.0 {
            MachOExecutableImageConversionState::Convertible { .. } => None,
            MachOExecutableImageConversionState::NotConvertible { blocker } => Some(blocker),
        }
    }

    pub const fn entry_point(&self) -> Option<RecognizedMachOEntryPointCommand> {
        match self.0 {
            MachOExecutableImageConversionState::Convertible { entry_point, .. } => {
                Some(entry_point)
            }
            MachOExecutableImageConversionState::NotConvertible { .. } => None,
        }
    }

    pub const fn segment(&self) -> Option<&RecognizedMachOSegmentCommand> {
        match &self.0 {
            MachOExecutableImageConversionState::Convertible { segment, .. } => Some(segment),
            MachOExecutableImageConversionState::NotConvertible { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum MachOExecutableImageConversionState {
    Convertible {
        entry_point: RecognizedMachOEntryPointCommand,
        segment: RecognizedMachOSegmentCommand,
    },
    NotConvertible {
        blocker: MachOExecutableImageConversionBlocker,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOExecutableImageConversionStatus {
    Convertible,
    NotConvertible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOExecutableImageConversionBlocker {
    MissingEntryPoint,
    AmbiguousEntryPoint,
    MissingSegment,
    EntryPointOutsideSegment,
    AmbiguousEntrySegment,
    UnsupportedImageMapping,
}

pub(crate) fn classify_mach_o_executable_image_conversion(
    summary: &MachOLoadCommandSummary,
) -> MachOExecutableImageConversion {
    let entry_point = match summary.recognized_entry_points() {
        [] => {
            return MachOExecutableImageConversion::not_convertible(
                MachOExecutableImageConversionBlocker::MissingEntryPoint,
            );
        }
        [_, _, ..] => {
            return MachOExecutableImageConversion::not_convertible(
                MachOExecutableImageConversionBlocker::AmbiguousEntryPoint,
            );
        }
        [entry_point] => *entry_point,
    };

    if summary.recognized_segments().is_empty() {
        return MachOExecutableImageConversion::not_convertible(
            MachOExecutableImageConversionBlocker::MissingSegment,
        );
    };

    let containing_segments = recognized_segments_containing_entry_point(summary, entry_point);
    match containing_segments.as_slice() {
        [] => MachOExecutableImageConversion::not_convertible(
            MachOExecutableImageConversionBlocker::EntryPointOutsideSegment,
        ),
        [segment] => MachOExecutableImageConversion::convertible(entry_point, (*segment).clone()),
        [_, _, ..] => MachOExecutableImageConversion::not_convertible(
            MachOExecutableImageConversionBlocker::AmbiguousEntrySegment,
        ),
    }
}

fn recognized_segments_containing_entry_point(
    summary: &MachOLoadCommandSummary,
    entry_point: RecognizedMachOEntryPointCommand,
) -> Vec<&RecognizedMachOSegmentCommand> {
    summary
        .recognized_segments()
        .iter()
        .filter(|segment| {
            segment
                .header()
                .contains_entry_point_file_offset(entry_point.metadata().entryoff())
        })
        .collect()
}
