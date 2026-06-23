use super::{
    GuestImage, GuestImageAddressSpace, GuestImageEntryPoint, GuestImageError, GuestImageFormat,
    GuestImageImports, GuestImageMappedBytes, GuestImageMappedBytesSource, GuestImageMetadata,
    GuestImageRelocations, GuestImageSections, GuestImageSegment, GuestImageSegmentKind,
    GuestImageSegmentSource, GuestImageSegments, GuestImageSymbols, GuestImageUnwindMetadata,
    MachOExecutableCodeRange, MachOExecutableCodeSegment, MachOExecutableEntryPoint, MachOImage,
};
use bara_ir::{
    ExternalSymbolId, ExternalSymbolImport, ProgramImageImport, ProgramImageImports,
    ProgramImageMappedByteSegment, ProgramImageMappedBytes, ProgramImageMetadata,
    ProgramImageRange, ProgramImageRelocation, ProgramImageRelocationTarget,
    ProgramImageRelocations, ProgramImageSection, ProgramImageSectionKind, ProgramImageSections,
    ProgramImageSymbol, ProgramImageSymbols, ProgramUnwindEntry, ProgramUnwindMetadata, X86Va,
};

fn image_range(start: u64, end: u64) -> ProgramImageRange {
    ProgramImageRange::new(X86Va::new(start), X86Va::new(end)).expect("test image range is valid")
}

fn code_segment() -> GuestImageSegment {
    GuestImageSegment::new(
        GuestImageSegmentKind::Code,
        image_range(0x1_0000_0000, 0x1_0000_1000),
        GuestImageSegmentSource::LcSegment64FileRange,
        GuestImageAddressSpace::MachOVirtualAddress,
    )
}

fn mach_o_code_segment() -> MachOExecutableCodeSegment {
    MachOExecutableCodeSegment::new(MachOExecutableCodeRange::new(image_range(
        0x1_0000_0000,
        0x1_0000_1000,
    )))
}

fn mapped_bytes() -> ProgramImageMappedBytes {
    let bytes = vec![42, 0, 0, 0, 0, 0, 0, 0];
    let segment =
        ProgramImageMappedByteSegment::new(image_range(0x1_0000_0000, 0x1_0000_0008), bytes)
            .expect("test mapped byte segment is valid");
    ProgramImageMappedBytes::from_segments([segment])
}

fn guest_image_mapped_bytes() -> GuestImageMappedBytes {
    GuestImageMappedBytes::new(
        GuestImageMappedBytesSource::ProgramImageMetadata,
        mapped_bytes(),
    )
}

fn imports() -> ProgramImageImports {
    let import = ExternalSymbolImport::unresolved(ExternalSymbolId::new(7));
    ProgramImageImports::from_items([ProgramImageImport::new(import)])
}

fn guest_image_imports() -> GuestImageImports {
    GuestImageImports::new(imports())
}

fn relocations() -> ProgramImageRelocations {
    ProgramImageRelocations::from_items([ProgramImageRelocation::new(
        X86Va::new(0x1_0000_0020),
        ProgramImageRelocationTarget::ExternalSymbol(ExternalSymbolId::new(7)),
    )])
}

fn guest_image_relocations() -> GuestImageRelocations {
    GuestImageRelocations::new(relocations())
}

fn sections() -> ProgramImageSections {
    ProgramImageSections::from_items([
        ProgramImageSection::new(
            ProgramImageSectionKind::Code,
            image_range(0x1_0000_0000, 0x1_0000_1000),
        ),
        ProgramImageSection::new(
            ProgramImageSectionKind::ConstData,
            image_range(0x1_0000_2000, 0x1_0000_2010),
        ),
    ])
}

fn guest_image_sections() -> GuestImageSections {
    GuestImageSections::new(sections())
}

fn symbols() -> ProgramImageSymbols {
    let import = ExternalSymbolImport::unresolved(ExternalSymbolId::new(7));
    ProgramImageSymbols::from_items([ProgramImageSymbol::external_import(import)])
}

fn guest_image_symbols() -> GuestImageSymbols {
    GuestImageSymbols::new(symbols())
}

fn unwind() -> ProgramUnwindMetadata {
    ProgramUnwindMetadata::from_entries([ProgramUnwindEntry::new(image_range(
        0x1_0000_0000,
        0x1_0000_0040,
    ))])
}

fn guest_image_unwind() -> GuestImageUnwindMetadata {
    GuestImageUnwindMetadata::new(unwind())
}

fn program_image_metadata() -> ProgramImageMetadata {
    ProgramImageMetadata::new_with_mapped_bytes(
        sections(),
        mapped_bytes(),
        symbols(),
        relocations(),
        imports(),
        unwind(),
    )
}

fn metadata() -> GuestImageMetadata {
    GuestImageMetadata::new(
        guest_image_mapped_bytes(),
        guest_image_sections(),
        guest_image_symbols(),
        guest_image_relocations(),
        guest_image_imports(),
        guest_image_unwind(),
    )
}

#[test]
fn mach_o_guest_image_exposes_runtime_facing_mapping_shell() {
    let image = GuestImage::mach_o_executable(
        GuestImageEntryPoint::new(X86Va::new(0x1_0000_0010)),
        code_segment(),
        metadata(),
    )
    .expect("entry is inside code segment");

    assert_eq!(image.entry_point().address(), X86Va::new(0x1_0000_0010));
    assert_eq!(image.code_segment(), Some(code_segment()));
    assert_eq!(
        image.mapped_bytes_source(),
        GuestImageMappedBytesSource::ProgramImageMetadata
    );
    assert_eq!(
        image.mapped_bytes().read_u64_le(X86Va::new(0x1_0000_0000)),
        Some(42)
    );
    assert_eq!(image.metadata(), &metadata());
    assert_eq!(image.sections().items().len(), 2);
    assert_eq!(
        image.sections().items()[1].kind(),
        ProgramImageSectionKind::ConstData
    );
    assert_eq!(image.imports().items().len(), 1);
    assert_eq!(
        image.imports().items()[0].import(),
        ExternalSymbolImport::unresolved(ExternalSymbolId::new(7))
    );
    assert_eq!(image.symbols().items().len(), 1);
    assert_eq!(
        image.symbols().items()[0],
        ProgramImageSymbol::external_import(ExternalSymbolImport::unresolved(
            ExternalSymbolId::new(7)
        ))
    );
    assert_eq!(image.relocations().items().len(), 1);
    assert_eq!(
        image.relocations().items()[0].target(),
        ProgramImageRelocationTarget::ExternalSymbol(ExternalSymbolId::new(7))
    );
    assert_eq!(image.unwind().entries().len(), 1);
    assert_eq!(
        image.unwind().entries()[0].range(),
        image_range(0x1_0000_0000, 0x1_0000_0040)
    );
}

#[test]
fn mach_o_image_wraps_runtime_guest_image_shell() {
    let image = MachOImage::executable(
        MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010)),
        mach_o_code_segment(),
        metadata(),
    )
    .expect("entry is inside code segment");

    assert_eq!(image.guest_image().format(), GuestImageFormat::MachO);
    assert_eq!(image.entry_point().address(), X86Va::new(0x1_0000_0010));
    assert_eq!(image.code_segment(), mach_o_code_segment());
    assert_eq!(image.metadata(), &metadata());
    assert_eq!(image.guest_image().metadata(), &metadata());
}

#[test]
fn mach_o_image_builds_executable_code_segment_from_range() {
    let image = MachOImage::executable_from_code_range(
        MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010)),
        MachOExecutableCodeRange::new(image_range(0x1_0000_0000, 0x1_0000_1000)),
        metadata(),
    )
    .expect("entry is inside code segment");

    assert_eq!(image.guest_image().format(), GuestImageFormat::MachO);
    assert_eq!(image.entry_point().address(), X86Va::new(0x1_0000_0010));
    assert_eq!(image.code_segment(), mach_o_code_segment());
    assert_eq!(image.metadata(), &metadata());
}

#[test]
fn mach_o_executable_code_segment_exposes_mapping_identity() {
    let segment = mach_o_code_segment();

    assert_eq!(
        segment.range(),
        MachOExecutableCodeRange::new(image_range(0x1_0000_0000, 0x1_0000_1000))
    );
    assert_eq!(
        segment.source(),
        GuestImageSegmentSource::LcSegment64FileRange
    );
    assert_eq!(
        segment.address_space(),
        GuestImageAddressSpace::MachOVirtualAddress
    );
    assert_eq!(segment.guest_image_segment(), code_segment());
}

#[test]
fn mach_o_executable_code_segment_exposes_derived_mapping_values() {
    let segment = mach_o_code_segment();

    assert_eq!(segment.vmaddr(), X86Va::new(0x1_0000_0000));
    assert_eq!(
        segment
            .byte_len()
            .expect("test code segment length is valid")
            .as_usize(),
        0x1000
    );
}

#[test]
fn guest_image_mapped_bytes_exposes_source_and_payload() {
    let mapped_bytes = guest_image_mapped_bytes();

    assert_eq!(
        mapped_bytes.source(),
        GuestImageMappedBytesSource::ProgramImageMetadata
    );
    assert_eq!(
        mapped_bytes
            .payload()
            .read_u64_le(X86Va::new(0x1_0000_0000)),
        Some(42)
    );
}

#[test]
fn guest_image_imports_exposes_payload() {
    let imports = guest_image_imports();

    assert_eq!(imports.payload().items().len(), 1);
    assert_eq!(
        imports.payload().items()[0].import(),
        ExternalSymbolImport::unresolved(ExternalSymbolId::new(7))
    );
}

#[test]
fn guest_image_relocations_exposes_payload() {
    let relocations = guest_image_relocations();

    assert_eq!(relocations.payload().items().len(), 1);
    assert_eq!(
        relocations.payload().items()[0].target(),
        ProgramImageRelocationTarget::ExternalSymbol(ExternalSymbolId::new(7))
    );
}

#[test]
fn guest_image_sections_exposes_payload() {
    let sections = guest_image_sections();

    assert_eq!(sections.payload().items().len(), 2);
    assert_eq!(
        sections.payload().items()[0].kind(),
        ProgramImageSectionKind::Code
    );
    assert_eq!(
        sections.payload().items()[1].range(),
        image_range(0x1_0000_2000, 0x1_0000_2010)
    );
}

#[test]
fn guest_image_symbols_exposes_payload() {
    let symbols = guest_image_symbols();

    assert_eq!(symbols.payload().items().len(), 1);
    assert_eq!(
        symbols.payload().items()[0],
        ProgramImageSymbol::external_import(ExternalSymbolImport::unresolved(
            ExternalSymbolId::new(7)
        ))
    );
}

#[test]
fn guest_image_unwind_metadata_exposes_payload() {
    let unwind = guest_image_unwind();

    assert_eq!(unwind.payload().entries().len(), 1);
    assert_eq!(
        unwind.payload().entries()[0].range(),
        image_range(0x1_0000_0000, 0x1_0000_0040)
    );
}

#[test]
fn mach_o_executable_code_range_exposes_program_image_range() {
    let range = image_range(0x1_0000_0000, 0x1_0000_1000);
    assert_eq!(MachOExecutableCodeRange::new(range).range(), range);
}

#[test]
fn mach_o_executable_entry_point_exposes_typed_address() {
    let entry_point = MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010));

    assert_eq!(entry_point.address(), X86Va::new(0x1_0000_0010));
    assert_eq!(
        entry_point.guest_image_entry_point(),
        GuestImageEntryPoint::new(X86Va::new(0x1_0000_0010))
    );
}

#[test]
fn mach_o_executable_code_range_uses_single_code_section_from_metadata() {
    assert_eq!(
        MachOExecutableCodeRange::from_program_image_metadata(&program_image_metadata()),
        Ok(MachOExecutableCodeRange::new(image_range(
            0x1_0000_0000,
            0x1_0000_1000
        )))
    );
}

#[test]
fn mach_o_executable_code_range_rejects_missing_code_section() {
    let metadata = ProgramImageMetadata::new_with_mapped_bytes(
        ProgramImageSections::from_items([ProgramImageSection::new(
            ProgramImageSectionKind::ConstData,
            image_range(0x1_0000_2000, 0x1_0000_2010),
        )]),
        mapped_bytes(),
        symbols(),
        relocations(),
        imports(),
        unwind(),
    );

    assert_eq!(
        MachOExecutableCodeRange::from_program_image_metadata(&metadata),
        Err(GuestImageError::MissingMachOExecutableCodeSection)
    );
}

#[test]
fn mach_o_executable_code_range_rejects_ambiguous_code_sections() {
    let metadata = ProgramImageMetadata::new_with_mapped_bytes(
        ProgramImageSections::from_items([
            ProgramImageSection::new(
                ProgramImageSectionKind::Code,
                image_range(0x1_0000_0000, 0x1_0000_1000),
            ),
            ProgramImageSection::new(
                ProgramImageSectionKind::Code,
                image_range(0x1_0000_2000, 0x1_0000_3000),
            ),
        ]),
        mapped_bytes(),
        symbols(),
        relocations(),
        imports(),
        unwind(),
    );

    assert_eq!(
        MachOExecutableCodeRange::from_program_image_metadata(&metadata),
        Err(GuestImageError::AmbiguousMachOExecutableCodeSections)
    );
}

#[test]
fn mach_o_image_builds_guest_metadata_from_program_image_metadata() {
    let program_metadata = program_image_metadata();
    let image = MachOImage::executable_from_program_image_metadata(
        MachOExecutableEntryPoint::new(X86Va::new(0x1_0000_0010)),
        &program_metadata,
    )
    .expect("entry is inside code segment");

    assert_eq!(
        image.guest_image().mapped_bytes_source(),
        GuestImageMappedBytesSource::ProgramImageMetadata
    );
    assert_eq!(image.code_segment(), mach_o_code_segment());
    assert_eq!(image.metadata(), &metadata());
}

#[test]
fn guest_image_rejects_entry_outside_mapped_segments() {
    assert_eq!(
        GuestImage::mach_o_executable(
            GuestImageEntryPoint::new(X86Va::new(0x1_0000_1000)),
            code_segment(),
            metadata(),
        ),
        Err(GuestImageError::EntryOutsideMappedSegments)
    );
}

#[test]
fn guest_image_rejects_missing_code_segment() {
    assert_eq!(
        GuestImage::new(
            super::GuestImageFormat::MachO,
            GuestImageEntryPoint::new(X86Va::new(0x1_0000_0010)),
            GuestImageSegments::empty(),
            metadata(),
        ),
        Err(GuestImageError::MissingCodeSegment)
    );
}
