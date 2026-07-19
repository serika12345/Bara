use bara_ir::ProgramImageMetadata;

use crate::GuestProgramCounter;

use super::{
    GuestImageError, GuestImageMappedBytesSource, MachOExecutableCodeByteLen,
    MachOExecutableCodeBytes, MachOExecutableCodeSegment, MachOExecutableImageSnapshot,
};

#[derive(Debug, Eq, PartialEq)]
pub struct MachOExecutableImagePreparation {
    snapshot: MachOExecutableImageSnapshot,
    executable_code_bytes: MachOExecutableCodeBytes,
    executable_code_byte_len: MachOExecutableCodeByteLen,
    initial_program_counter: GuestProgramCounter,
}

impl MachOExecutableImagePreparation {
    pub fn try_from_snapshot(
        snapshot: MachOExecutableImageSnapshot,
    ) -> Result<Self, GuestImageError> {
        let executable_code_bytes = snapshot.executable_code_bytes()?;
        let executable_code_byte_len = snapshot.mapping().code_segment().byte_len()?;
        let initial_program_counter =
            GuestProgramCounter::new(snapshot.mapping().entry_point().address());

        Ok(Self {
            snapshot,
            executable_code_bytes,
            executable_code_byte_len,
            initial_program_counter,
        })
    }

    pub const fn executable_code_bytes(&self) -> &MachOExecutableCodeBytes {
        &self.executable_code_bytes
    }

    pub const fn initial_program_counter(&self) -> GuestProgramCounter {
        self.initial_program_counter
    }

    pub const fn executable_code_byte_len(&self) -> MachOExecutableCodeByteLen {
        self.executable_code_byte_len
    }

    pub const fn code_segment(&self) -> MachOExecutableCodeSegment {
        self.snapshot.mapping().code_segment()
    }

    pub const fn mapped_bytes_source(&self) -> GuestImageMappedBytesSource {
        self.snapshot.mapping().mapped_bytes_source()
    }

    pub fn program_image_metadata(&self) -> ProgramImageMetadata {
        self.snapshot.program_image_metadata()
    }
}
