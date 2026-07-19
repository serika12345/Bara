mod image_metadata;

pub use image_metadata::{
    ProgramImageImport, ProgramImageImports, ProgramImageMappedByteSegment,
    ProgramImageMappedBytes, ProgramImageMetadata, ProgramImageMetadataError, ProgramImageRange,
    ProgramImageRelocation, ProgramImageRelocationTarget, ProgramImageRelocations,
    ProgramImageSection, ProgramImageSectionKind, ProgramImageSections, ProgramImageSymbol,
    ProgramImageSymbols, ProgramUnwindEntry, ProgramUnwindMetadata,
};

use crate::block::{BasicBlock, BlockId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct X86Va(u64);

impl X86Va {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, byte_len: u64) -> Result<Self, ProgramError> {
        self.0
            .checked_add(byte_len)
            .map(Self)
            .ok_or(ProgramError::AddressOverflow {
                start: self,
                byte_len,
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    entry: X86Va,
    blocks: Vec<BasicBlock>,
    image_metadata: ProgramImageMetadata,
}

impl Program {
    pub fn new(entry: X86Va, blocks: Vec<BasicBlock>) -> Result<Self, ProgramError> {
        Self::with_image_metadata(entry, blocks, ProgramImageMetadata::empty())
    }

    pub fn with_image_metadata(
        entry: X86Va,
        blocks: Vec<BasicBlock>,
        image_metadata: ProgramImageMetadata,
    ) -> Result<Self, ProgramError> {
        let has_entry = blocks.iter().any(|block| block.start() == entry);
        if !has_entry {
            return Err(ProgramError::MissingEntryBlock { entry });
        }

        let mut seen = Vec::new();
        for block in &blocks {
            if seen.contains(&block.id()) {
                return Err(ProgramError::DuplicateBlockId { id: block.id() });
            }
            seen.push(block.id());
        }

        Ok(Self {
            entry,
            blocks,
            image_metadata,
        })
    }

    pub const fn entry(&self) -> X86Va {
        self.entry
    }

    pub fn blocks(&self) -> &[BasicBlock] {
        &self.blocks
    }

    pub const fn image_metadata(&self) -> &ProgramImageMetadata {
        &self.image_metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgramError {
    AddressOverflow { start: X86Va, byte_len: u64 },
    MissingEntryBlock { entry: X86Va },
    DuplicateBlockId { id: BlockId },
}

#[cfg(test)]
mod tests {
    use crate::{
        BasicBlock, BlockId, ExternalSymbolId, ExternalSymbolImport, Program, ProgramError,
        ProgramImageImport, ProgramImageImports, ProgramImageMappedByteSegment,
        ProgramImageMappedBytes, ProgramImageMetadata, ProgramImageMetadataError,
        ProgramImageRange, ProgramImageRelocation, ProgramImageRelocationTarget,
        ProgramImageRelocations, ProgramImageSection, ProgramImageSectionKind,
        ProgramImageSections, ProgramImageSymbol, ProgramImageSymbols, ProgramUnwindEntry,
        ProgramUnwindMetadata, Terminator, X86Va,
    };

    fn block(id: u32, start: u64, end: u64) -> BasicBlock {
        BasicBlock::new(
            BlockId::new(id),
            X86Va::new(start),
            X86Va::new(end),
            Vec::new(),
            Terminator::Return,
        )
        .expect("test block range is valid")
    }

    #[test]
    fn x86_va_checked_add_returns_typed_address() {
        assert_eq!(X86Va::new(0x1000).checked_add(5), Ok(X86Va::new(0x1005)));
    }

    #[test]
    fn x86_va_checked_add_reports_overflow() {
        assert_eq!(
            X86Va::new(u64::MAX).checked_add(1),
            Err(ProgramError::AddressOverflow {
                start: X86Va::new(u64::MAX),
                byte_len: 1
            })
        );
    }

    #[test]
    fn program_requires_entry_block() {
        assert_eq!(
            Program::new(X86Va::new(0), vec![block(0, 1, 2)]),
            Err(ProgramError::MissingEntryBlock {
                entry: X86Va::new(0)
            })
        );
    }

    #[test]
    fn program_rejects_duplicate_block_id() {
        assert_eq!(
            Program::new(X86Va::new(0), vec![block(0, 0, 1), block(0, 1, 2)]),
            Err(ProgramError::DuplicateBlockId {
                id: BlockId::new(0)
            })
        );
    }

    #[test]
    fn program_exposes_entry_and_blocks() {
        let program = Program::new(X86Va::new(0), vec![block(7, 0, 1)])
            .expect("program has entry block and unique block id");

        assert_eq!(program.entry(), X86Va::new(0));
        assert_eq!(program.blocks()[0].id(), BlockId::new(7));
        assert!(program.image_metadata().is_empty());
    }

    #[test]
    fn program_preserves_image_metadata_collections() {
        let code_range = ProgramImageRange::new(X86Va::new(0x1000), X86Va::new(0x1008))
            .expect("range is non-empty");
        let mapped_segment = ProgramImageMappedByteSegment::new(
            code_range,
            vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11],
        )
        .expect("mapped bytes match range");
        let import = ExternalSymbolImport::unresolved(ExternalSymbolId::new(7));
        let metadata = ProgramImageMetadata::new_with_mapped_bytes(
            ProgramImageSections::from_items([ProgramImageSection::new(
                ProgramImageSectionKind::Code,
                code_range,
            )]),
            ProgramImageMappedBytes::from_segments([mapped_segment]),
            ProgramImageSymbols::from_items([ProgramImageSymbol::external_import(import)]),
            ProgramImageRelocations::from_items([ProgramImageRelocation::new(
                X86Va::new(0x1002),
                ProgramImageRelocationTarget::ExternalSymbol(ExternalSymbolId::new(7)),
            )]),
            ProgramImageImports::from_items([ProgramImageImport::new(import)]),
            ProgramUnwindMetadata::from_entries([ProgramUnwindEntry::new(code_range)]),
        );

        let program =
            Program::with_image_metadata(X86Va::new(0), vec![block(7, 0, 1)], metadata.clone())
                .expect("program has entry block and metadata");

        assert_eq!(program.image_metadata(), &metadata);
        assert_eq!(
            program.image_metadata().sections().items()[0].kind(),
            ProgramImageSectionKind::Code
        );
        assert_eq!(
            program.image_metadata().relocations().items()[0].target(),
            ProgramImageRelocationTarget::ExternalSymbol(ExternalSymbolId::new(7))
        );
        assert_eq!(
            program
                .image_metadata()
                .mapped_bytes()
                .read_u64_le(X86Va::new(0x1000)),
            Some(0x1122_3344_5566_7788)
        );
    }

    #[test]
    fn mapped_image_bytes_reject_length_mismatch() {
        let range = ProgramImageRange::new(X86Va::new(0x1000), X86Va::new(0x1008))
            .expect("range is non-empty");

        assert_eq!(
            ProgramImageMappedByteSegment::new(range, vec![0; 7]),
            Err(ProgramImageMetadataError::MappedBytesLengthMismatch)
        );
    }

    #[test]
    fn mapped_image_bytes_return_none_for_out_of_range_qword() {
        let range = ProgramImageRange::new(X86Va::new(0x1000), X86Va::new(0x1008))
            .expect("range is non-empty");
        let mapped_segment = ProgramImageMappedByteSegment::new(range, vec![0; 8])
            .expect("mapped bytes match range");
        let mapped_bytes = ProgramImageMappedBytes::from_segments([mapped_segment]);

        assert_eq!(mapped_bytes.read_u64_le(X86Va::new(0x0fff)), None);
        assert_eq!(mapped_bytes.read_u64_le(X86Va::new(0x1001)), None);
    }

    #[test]
    fn mapped_image_bytes_extract_typed_segment_for_source_range() {
        let mapped_range = ProgramImageRange::new(X86Va::new(0x1000), X86Va::new(0x1010))
            .expect("range is non-empty");
        let source_range = ProgramImageRange::new(X86Va::new(0x1004), X86Va::new(0x100c))
            .expect("range is non-empty");
        let mapped_segment = ProgramImageMappedByteSegment::new(
            mapped_range,
            vec![
                0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d,
                0x9e, 0x9f,
            ],
        )
        .expect("mapped bytes match range");
        let mapped_bytes = ProgramImageMappedBytes::from_segments([mapped_segment]);
        let expected = ProgramImageMappedByteSegment::new(
            source_range,
            vec![0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b],
        )
        .expect("expected bytes match source range");

        assert_eq!(mapped_bytes.segment_for_range(source_range), Ok(expected));
    }

    #[test]
    fn mapped_image_bytes_reject_unavailable_source_range() {
        let mapped_range = ProgramImageRange::new(X86Va::new(0x1000), X86Va::new(0x1008))
            .expect("range is non-empty");
        let unavailable_range = ProgramImageRange::new(X86Va::new(0x1004), X86Va::new(0x100c))
            .expect("range is non-empty");
        let mapped_segment = ProgramImageMappedByteSegment::new(mapped_range, vec![0; 8])
            .expect("mapped bytes match range");
        let mapped_bytes = ProgramImageMappedBytes::from_segments([mapped_segment]);

        assert_eq!(
            mapped_bytes.segment_for_range(unavailable_range),
            Err(ProgramImageMetadataError::MappedBytesRangeUnavailable)
        );
    }

    #[test]
    fn mapped_image_bytes_read_nul_terminated_utf8_by_vm_address() {
        let range = ProgramImageRange::new(X86Va::new(0x1000), X86Va::new(0x1010))
            .expect("range is non-empty");
        let mapped_segment =
            ProgramImageMappedByteSegment::new(range, b"prefix\0shared\0xx".to_vec())
                .expect("mapped bytes match range");
        let mapped_bytes = ProgramImageMappedBytes::from_segments([mapped_segment]);

        assert_eq!(
            mapped_bytes.read_nul_terminated_utf8(X86Va::new(0x1000)),
            Some("prefix")
        );
        assert_eq!(
            mapped_bytes.read_nul_terminated_utf8(X86Va::new(0x1007)),
            Some("shared")
        );
        assert_eq!(
            mapped_bytes.read_nul_terminated_utf8(X86Va::new(0x100e)),
            None
        );
        assert_eq!(
            mapped_bytes.read_nul_terminated_utf8(X86Va::new(0x1010)),
            None
        );
    }
}
