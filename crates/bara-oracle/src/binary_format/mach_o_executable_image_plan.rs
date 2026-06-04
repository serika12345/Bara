use super::{
    mach_o_entry_point_command::MachOEntryPointFileOffset,
    mach_o_executable_image_conversion::{
        MachOExecutableImageConversion, MachOExecutableImageConversionBlocker,
    },
    mach_o_segment_command::{MachOSegmentFileOffset, MachOSegmentFileSize},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOExecutableImagePlan {
    segment_file_range: MachOSegmentFileRange,
    entry_point_segment_offset: MachOEntryPointSegmentOffset,
}

impl MachOExecutableImagePlan {
    pub(crate) const fn new(
        segment_file_range: MachOSegmentFileRange,
        entry_point_segment_offset: MachOEntryPointSegmentOffset,
    ) -> Self {
        Self {
            segment_file_range,
            entry_point_segment_offset,
        }
    }

    pub const fn segment_file_range(&self) -> MachOSegmentFileRange {
        self.segment_file_range
    }

    pub const fn entry_point_segment_offset(&self) -> MachOEntryPointSegmentOffset {
        self.entry_point_segment_offset
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOSegmentFileRange {
    offset: MachOSegmentFileOffset,
    size: MachOSegmentFileSize,
}

impl MachOSegmentFileRange {
    pub(crate) const fn new(offset: MachOSegmentFileOffset, size: MachOSegmentFileSize) -> Self {
        Self { offset, size }
    }

    pub const fn offset(self) -> MachOSegmentFileOffset {
        self.offset
    }

    pub const fn size(self) -> MachOSegmentFileSize {
        self.size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOEntryPointSegmentOffset {
    value: u64,
}

impl MachOEntryPointSegmentOffset {
    pub(crate) const fn from_valid_segment_relative_value(value: u64) -> Self {
        Self { value }
    }

    fn from_file_offsets(
        entry_point_file_offset: MachOEntryPointFileOffset,
        segment_file_offset: MachOSegmentFileOffset,
    ) -> Result<Self, MachOExecutableImagePlanError> {
        let Some(value) = entry_point_file_offset
            .as_u64()
            .checked_sub(segment_file_offset.as_u64())
        else {
            return Err(MachOExecutableImagePlanError::EntryPointBeforeSegmentFileRange);
        };

        Ok(Self::from_valid_segment_relative_value(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOExecutableImagePlanError {
    NotConvertible {
        blocker: MachOExecutableImageConversionBlocker,
    },
    EntryPointBeforeSegmentFileRange,
}

pub fn plan_mach_o_executable_image(
    conversion: &MachOExecutableImageConversion,
) -> Result<MachOExecutableImagePlan, MachOExecutableImagePlanError> {
    let (entry_point, segment) = conversion
        .selected_candidate()
        .map_err(|blocker| MachOExecutableImagePlanError::NotConvertible { blocker })?;

    let segment_file_range =
        MachOSegmentFileRange::new(segment.header().fileoff(), segment.header().filesize());
    let entry_point_segment_offset = MachOEntryPointSegmentOffset::from_file_offsets(
        entry_point.metadata().entryoff(),
        segment.header().fileoff(),
    )?;

    Ok(MachOExecutableImagePlan::new(
        segment_file_range,
        entry_point_segment_offset,
    ))
}
