use bara_ir::X86Va;
use bara_isa_x86::{DecodeError, X86Bytes};

use crate::executable_manifest::{
    CodeSegment, ExecutableEntry, ExecutableImage, ExecutableImageError,
};

use super::mach_o_executable_image_plan::MachOExecutableImagePlan;
use crate::binary_format::input::BinaryInput;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachOExecutableImageMaterializationError {
    SegmentFileRangeOutOfBounds,
    DecodeInput(DecodeError),
    ExecutableImage(ExecutableImageError),
}

pub fn materialize_mach_o_executable_image(
    input: &BinaryInput,
    plan: &MachOExecutableImagePlan,
) -> Result<ExecutableImage, MachOExecutableImageMaterializationError> {
    let range = plan.segment_file_range();
    let offset = usize::try_from(range.offset().as_u64())
        .map_err(|_| MachOExecutableImageMaterializationError::SegmentFileRangeOutOfBounds)?;
    let size = usize::try_from(range.size().as_u64())
        .map_err(|_| MachOExecutableImageMaterializationError::SegmentFileRangeOutOfBounds)?;
    let bytes = input
        .read_bytes_at(offset, size)
        .ok_or(MachOExecutableImageMaterializationError::SegmentFileRangeOutOfBounds)?
        .to_vec();

    let code_segment = CodeSegment::from_x86_bytes(
        X86Bytes::new(X86Va::new(plan.segment_vmaddr().as_u64()), bytes)
            .map_err(MachOExecutableImageMaterializationError::DecodeInput)?,
    );
    let entry = ExecutableEntry::new(X86Va::new(plan.entry_point_virtual_address().as_u64()));

    ExecutableImage::new(code_segment, entry)
        .map_err(MachOExecutableImageMaterializationError::ExecutableImage)
}
