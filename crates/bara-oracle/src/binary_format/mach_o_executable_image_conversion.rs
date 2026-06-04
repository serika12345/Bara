use super::mach_o_load_command::MachOLoadCommandSummary;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MachOExecutableImageConversion {
    status: MachOExecutableImageConversionStatus,
    blocker: MachOExecutableImageConversionBlocker,
}

impl MachOExecutableImageConversion {
    pub const fn not_convertible(blocker: MachOExecutableImageConversionBlocker) -> Self {
        Self {
            status: MachOExecutableImageConversionStatus::NotConvertible,
            blocker,
        }
    }

    pub const fn status(self) -> MachOExecutableImageConversionStatus {
        self.status
    }

    pub const fn blocker(self) -> MachOExecutableImageConversionBlocker {
        self.blocker
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachOExecutableImageConversionStatus {
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
    let blocker = match summary.recognized_entry_points() {
        [] => MachOExecutableImageConversionBlocker::MissingEntryPoint,
        [_, _, ..] => MachOExecutableImageConversionBlocker::AmbiguousEntryPoint,
        [_] if summary.recognized_segments().is_empty() => {
            MachOExecutableImageConversionBlocker::MissingSegment
        }
        [_] => match recognized_segments_containing_entry_point(summary) {
            0 => MachOExecutableImageConversionBlocker::EntryPointOutsideSegment,
            1 => MachOExecutableImageConversionBlocker::UnsupportedImageMapping,
            _ => MachOExecutableImageConversionBlocker::AmbiguousEntrySegment,
        },
    };

    MachOExecutableImageConversion::not_convertible(blocker)
}

fn recognized_segments_containing_entry_point(summary: &MachOLoadCommandSummary) -> usize {
    let Some(entry_point) = summary.recognized_entry_points().first() else {
        return 0;
    };

    summary
        .recognized_segments()
        .iter()
        .filter(|segment| {
            segment
                .header()
                .contains_entry_point_file_offset(entry_point.metadata().entryoff())
        })
        .count()
}
