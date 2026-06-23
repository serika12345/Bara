use bara_ir::{
    ProgramImageImports, ProgramImageMappedBytes, ProgramImageMetadata, ProgramImageRelocations,
    ProgramImageSections, ProgramImageSymbols, ProgramUnwindMetadata,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestImageMappedBytesSource {
    ProgramImageMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageMappedBytes {
    source: GuestImageMappedBytesSource,
    payload: ProgramImageMappedBytes,
}

impl GuestImageMappedBytes {
    pub const fn new(
        source: GuestImageMappedBytesSource,
        payload: ProgramImageMappedBytes,
    ) -> Self {
        Self { source, payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(
            GuestImageMappedBytesSource::ProgramImageMetadata,
            metadata.mapped_bytes().clone(),
        )
    }

    pub const fn source(&self) -> GuestImageMappedBytesSource {
        self.source
    }

    pub const fn payload(&self) -> &ProgramImageMappedBytes {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageMetadata {
    mapped_bytes: GuestImageMappedBytes,
    sections: GuestImageSections,
    symbols: GuestImageSymbols,
    relocations: GuestImageRelocations,
    imports: GuestImageImports,
    unwind: GuestImageUnwindMetadata,
}

impl GuestImageMetadata {
    pub const fn new(
        mapped_bytes: GuestImageMappedBytes,
        sections: GuestImageSections,
        symbols: GuestImageSymbols,
        relocations: GuestImageRelocations,
        imports: GuestImageImports,
        unwind: GuestImageUnwindMetadata,
    ) -> Self {
        Self {
            mapped_bytes,
            sections,
            symbols,
            relocations,
            imports,
            unwind,
        }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(
            GuestImageMappedBytes::from_program_image_metadata(metadata),
            GuestImageSections::from_program_image_metadata(metadata),
            GuestImageSymbols::from_program_image_metadata(metadata),
            GuestImageRelocations::from_program_image_metadata(metadata),
            GuestImageImports::from_program_image_metadata(metadata),
            GuestImageUnwindMetadata::from_program_image_metadata(metadata),
        )
    }

    pub const fn mapped_bytes_source(&self) -> GuestImageMappedBytesSource {
        self.mapped_bytes.source()
    }

    pub const fn mapped_bytes_value(&self) -> &GuestImageMappedBytes {
        &self.mapped_bytes
    }

    pub const fn sections_value(&self) -> &GuestImageSections {
        &self.sections
    }

    pub const fn symbols_value(&self) -> &GuestImageSymbols {
        &self.symbols
    }

    pub const fn relocations_value(&self) -> &GuestImageRelocations {
        &self.relocations
    }

    pub const fn imports_value(&self) -> &GuestImageImports {
        &self.imports
    }

    pub const fn unwind_value(&self) -> &GuestImageUnwindMetadata {
        &self.unwind
    }

    pub const fn sections(&self) -> &ProgramImageSections {
        self.sections.payload()
    }

    pub const fn mapped_bytes(&self) -> &ProgramImageMappedBytes {
        self.mapped_bytes.payload()
    }

    pub const fn symbols(&self) -> &ProgramImageSymbols {
        self.symbols.payload()
    }

    pub const fn relocations(&self) -> &ProgramImageRelocations {
        self.relocations.payload()
    }

    pub const fn imports(&self) -> &ProgramImageImports {
        self.imports.payload()
    }

    pub const fn unwind(&self) -> &ProgramUnwindMetadata {
        self.unwind.payload()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageSections {
    payload: ProgramImageSections,
}

impl GuestImageSections {
    pub const fn new(payload: ProgramImageSections) -> Self {
        Self { payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(metadata.sections().clone())
    }

    pub const fn payload(&self) -> &ProgramImageSections {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageImports {
    payload: ProgramImageImports,
}

impl GuestImageImports {
    pub const fn new(payload: ProgramImageImports) -> Self {
        Self { payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(metadata.imports().clone())
    }

    pub const fn payload(&self) -> &ProgramImageImports {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageRelocations {
    payload: ProgramImageRelocations,
}

impl GuestImageRelocations {
    pub const fn new(payload: ProgramImageRelocations) -> Self {
        Self { payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(metadata.relocations().clone())
    }

    pub const fn payload(&self) -> &ProgramImageRelocations {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageSymbols {
    payload: ProgramImageSymbols,
}

impl GuestImageSymbols {
    pub const fn new(payload: ProgramImageSymbols) -> Self {
        Self { payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(metadata.symbols().clone())
    }

    pub const fn payload(&self) -> &ProgramImageSymbols {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestImageUnwindMetadata {
    payload: ProgramUnwindMetadata,
}

impl GuestImageUnwindMetadata {
    pub const fn new(payload: ProgramUnwindMetadata) -> Self {
        Self { payload }
    }

    pub fn from_program_image_metadata(metadata: &ProgramImageMetadata) -> Self {
        Self::new(metadata.unwind().clone())
    }

    pub const fn payload(&self) -> &ProgramUnwindMetadata {
        &self.payload
    }
}
