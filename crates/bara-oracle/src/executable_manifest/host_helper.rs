use serde::Deserialize;

use crate::TestCaseHostTrapPlan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostHelperImportTable {
    write_stdout: Option<HostHelperImport>,
}

impl HostHelperImportTable {
    pub const fn empty() -> Self {
        Self { write_stdout: None }
    }

    pub const fn write_stdout(&self) -> Option<&HostHelperImport> {
        self.write_stdout.as_ref()
    }

    fn insert(self, import: HostHelperImport) -> Result<Self, HostHelperImportTableError> {
        match import.name() {
            HostHelperName::WriteStdout => self.insert_write_stdout(import),
        }
    }

    fn insert_write_stdout(
        mut self,
        import: HostHelperImport,
    ) -> Result<Self, HostHelperImportTableError> {
        if self.write_stdout.is_some() {
            return Err(HostHelperImportTableError::DuplicateImport {
                helper: HostHelperName::WriteStdout,
            });
        }
        if import.signature() != HostHelperSignature::PtrLenToUnit {
            return Err(HostHelperImportTableError::SignatureMismatch {
                helper: HostHelperName::WriteStdout,
                expected: HostHelperSignature::PtrLenToUnit,
                actual: import.signature(),
            });
        }

        self.write_stdout = Some(import);
        Ok(self)
    }
}

impl Default for HostHelperImportTable {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostHelperImport {
    name: HostHelperName,
    signature: HostHelperSignature,
}

impl HostHelperImport {
    const fn new(name: HostHelperName, signature: HostHelperSignature) -> Self {
        Self { name, signature }
    }

    pub const fn name(self) -> HostHelperName {
        self.name
    }

    pub const fn signature(self) -> HostHelperSignature {
        self.signature
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostHelperName {
    WriteStdout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostHelperSignature {
    PtrLenToUnit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostHelperImportTableError {
    DuplicateImport {
        helper: HostHelperName,
    },
    SignatureMismatch {
        helper: HostHelperName,
        expected: HostHelperSignature,
        actual: HostHelperSignature,
    },
    MissingRequiredImport {
        helper: HostHelperName,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "kind")]
pub(crate) enum ExecutableImportDto {
    HostHelper {
        name: HostHelperNameDto,
        signature: HostHelperSignatureDto,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HostHelperNameDto {
    WriteStdout,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HostHelperSignatureDto {
    PtrLenToUnit,
}

pub(crate) fn host_helper_import_table_from_dtos(
    imports: Vec<ExecutableImportDto>,
) -> Result<HostHelperImportTable, HostHelperImportTableError> {
    imports.into_iter().map(HostHelperImport::from).try_fold(
        HostHelperImportTable::empty(),
        HostHelperImportTable::insert,
    )
}

pub(crate) fn validate_host_trap_imports(
    host_trap_plan: &TestCaseHostTrapPlan,
    import_table: &HostHelperImportTable,
) -> Result<(), HostHelperImportTableError> {
    if host_trap_plan.stdout_trap().is_some() && import_table.write_stdout().is_none() {
        return Err(HostHelperImportTableError::MissingRequiredImport {
            helper: HostHelperName::WriteStdout,
        });
    }

    Ok(())
}

impl From<ExecutableImportDto> for HostHelperImport {
    fn from(dto: ExecutableImportDto) -> Self {
        match dto {
            ExecutableImportDto::HostHelper { name, signature } => {
                Self::new(name.into(), signature.into())
            }
        }
    }
}

impl From<HostHelperNameDto> for HostHelperName {
    fn from(dto: HostHelperNameDto) -> Self {
        match dto {
            HostHelperNameDto::WriteStdout => Self::WriteStdout,
        }
    }
}

impl From<HostHelperSignatureDto> for HostHelperSignature {
    fn from(dto: HostHelperSignatureDto) -> Self {
        match dto {
            HostHelperSignatureDto::PtrLenToUnit => Self::PtrLenToUnit,
        }
    }
}
