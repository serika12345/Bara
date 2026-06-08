#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64MainCode {
    bytes: Box<[u8]>,
}

impl MachOArm64MainCode {
    pub fn from_emitted_code_bytes<T>(
        bytes: T,
    ) -> Result<Self, MachOArm64ExecutableWriterInputError>
    where
        T: Into<Box<[u8]>>,
    {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(MachOArm64ExecutableWriterInputError::EmptyMainCode);
        }

        Ok(Self { bytes })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64ConstData {
    bytes: Box<[u8]>,
}

impl MachOArm64ConstData {
    pub fn from_read_only_section_bytes<T>(
        bytes: T,
    ) -> Result<Self, MachOArm64ExecutableWriterInputError>
    where
        T: Into<Box<[u8]>>,
    {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(MachOArm64ExecutableWriterInputError::EmptyConstData);
        }

        Ok(Self { bytes })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachOArm64ExecutablePayload {
    MainOnly(MachOArm64MainCode),
    MainWithConstData {
        main: MachOArm64MainCode,
        const_data: MachOArm64ConstData,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64ExecutableWriterRequest {
    payload: MachOArm64ExecutablePayload,
}

impl MachOArm64ExecutableWriterRequest {
    pub fn main_only(main: MachOArm64MainCode) -> Self {
        Self {
            payload: MachOArm64ExecutablePayload::MainOnly(main),
        }
    }

    pub fn main_with_const_data(main: MachOArm64MainCode, const_data: MachOArm64ConstData) -> Self {
        Self {
            payload: MachOArm64ExecutablePayload::MainWithConstData { main, const_data },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64ExecutableWriterPlan {
    target: MachOArm64ExecutableTarget,
    payload: MachOArm64ExecutablePayload,
}

impl MachOArm64ExecutableWriterPlan {
    pub const fn target(&self) -> MachOArm64ExecutableTarget {
        self.target
    }

    pub const fn payload(&self) -> &MachOArm64ExecutablePayload {
        &self.payload
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOArm64ExecutableTarget {
    Arm64AppleMacos,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOArm64ExecutableWriterInputError {
    EmptyMainCode,
    EmptyConstData,
}

pub fn plan_mach_o_arm64_executable(
    request: MachOArm64ExecutableWriterRequest,
) -> MachOArm64ExecutableWriterPlan {
    MachOArm64ExecutableWriterPlan {
        target: MachOArm64ExecutableTarget::Arm64AppleMacos,
        payload: request.payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_arm64_main_executable_as_pure_writer_plan() {
        let main = MachOArm64MainCode::from_emitted_code_bytes([
            0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6,
        ])
        .expect("main code is non-empty");
        let request = MachOArm64ExecutableWriterRequest::main_only(main.clone());

        let plan = plan_mach_o_arm64_executable(request.clone());

        assert_eq!(plan.target(), MachOArm64ExecutableTarget::Arm64AppleMacos);
        assert_eq!(plan.payload(), &MachOArm64ExecutablePayload::MainOnly(main));
        assert_eq!(plan, plan_mach_o_arm64_executable(request));
    }

    #[test]
    fn plans_arm64_main_executable_with_const_payload() {
        let main = MachOArm64MainCode::from_emitted_code_bytes([
            0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6,
        ])
        .expect("main code is non-empty");
        let const_data = MachOArm64ConstData::from_read_only_section_bytes(*b"hello world\n")
            .expect("const data is non-empty");
        let request = MachOArm64ExecutableWriterRequest::main_with_const_data(
            main.clone(),
            const_data.clone(),
        );

        let plan = plan_mach_o_arm64_executable(request);

        assert_eq!(
            plan.payload(),
            &MachOArm64ExecutablePayload::MainWithConstData { main, const_data }
        );
    }

    #[test]
    fn rejects_empty_arm64_writer_payload_parts() {
        assert_eq!(
            MachOArm64MainCode::from_emitted_code_bytes(Vec::<u8>::new()),
            Err(MachOArm64ExecutableWriterInputError::EmptyMainCode)
        );
        assert_eq!(
            MachOArm64ConstData::from_read_only_section_bytes(Vec::<u8>::new()),
            Err(MachOArm64ExecutableWriterInputError::EmptyConstData)
        );
    }
}
