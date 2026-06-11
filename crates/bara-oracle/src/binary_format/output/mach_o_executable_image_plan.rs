use super::super::input::{
    MachOEntryPointFileOffset, MachOExecutableImageConversion,
    MachOExecutableImageConversionBlocker, MachOSegmentFileOffset, MachOSegmentFileSize,
    MachOSegmentVmAddr,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOExecutableImagePlan {
    segment_file_range: MachOSegmentFileRange,
    segment_vmaddr: MachOSegmentVmAddr,
    entry_point_segment_offset: MachOEntryPointSegmentOffset,
    entry_point_virtual_address: MachOEntryPointVirtualAddress,
}

impl MachOExecutableImagePlan {
    pub(crate) const fn new(
        segment_file_range: MachOSegmentFileRange,
        segment_vmaddr: MachOSegmentVmAddr,
        entry_point_segment_offset: MachOEntryPointSegmentOffset,
        entry_point_virtual_address: MachOEntryPointVirtualAddress,
    ) -> Self {
        Self {
            segment_file_range,
            segment_vmaddr,
            entry_point_segment_offset,
            entry_point_virtual_address,
        }
    }

    pub const fn segment_file_range(&self) -> MachOSegmentFileRange {
        self.segment_file_range
    }

    pub const fn segment_vmaddr(&self) -> MachOSegmentVmAddr {
        self.segment_vmaddr
    }

    pub const fn entry_point_segment_offset(&self) -> MachOEntryPointSegmentOffset {
        self.entry_point_segment_offset
    }

    pub const fn entry_point_virtual_address(&self) -> MachOEntryPointVirtualAddress {
        self.entry_point_virtual_address
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

    pub(crate) const fn as_u64(self) -> u64 {
        self.value
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
pub struct MachOEntryPointVirtualAddress {
    value: u64,
}

impl MachOEntryPointVirtualAddress {
    pub(crate) const fn from_valid_runtime_value(value: u64) -> Self {
        Self { value }
    }

    pub(crate) const fn as_u64(self) -> u64 {
        self.value
    }

    fn from_segment_vmaddr_and_offset(
        segment_vmaddr: MachOSegmentVmAddr,
        entry_point_segment_offset: MachOEntryPointSegmentOffset,
    ) -> Result<Self, MachOExecutableImagePlanError> {
        let Some(value) = segment_vmaddr
            .as_u64()
            .checked_add(entry_point_segment_offset.as_u64())
        else {
            return Err(MachOExecutableImagePlanError::EntryPointVirtualAddressOverflow);
        };

        Ok(Self::from_valid_runtime_value(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOExecutableImagePlanError {
    NotConvertible {
        blocker: MachOExecutableImageConversionBlocker,
    },
    EntryPointBeforeSegmentFileRange,
    EntryPointVirtualAddressOverflow,
}

pub fn plan_mach_o_executable_image(
    conversion: &MachOExecutableImageConversion,
) -> Result<MachOExecutableImagePlan, MachOExecutableImagePlanError> {
    let (entry_point, segment) = conversion
        .selected_candidate()
        .map_err(|blocker| MachOExecutableImagePlanError::NotConvertible { blocker })?;

    let segment_file_range =
        MachOSegmentFileRange::new(segment.header().fileoff(), segment.header().filesize());
    let segment_vmaddr = segment.header().vmaddr();
    let entry_point_segment_offset = MachOEntryPointSegmentOffset::from_file_offsets(
        entry_point.metadata().entryoff(),
        segment.header().fileoff(),
    )?;
    let entry_point_virtual_address =
        MachOEntryPointVirtualAddress::from_segment_vmaddr_and_offset(
            segment_vmaddr,
            entry_point_segment_offset,
        )?;

    Ok(MachOExecutableImagePlan::new(
        segment_file_range,
        segment_vmaddr,
        entry_point_segment_offset,
        entry_point_virtual_address,
    ))
}
