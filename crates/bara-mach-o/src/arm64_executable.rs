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
pub struct MachOArm64ExecutableModel {
    entry_point: MachOArm64EntryPoint,
    text_segment: MachOArm64TextSegment,
    load_commands: MachOArm64LoadCommands,
}

impl MachOArm64ExecutableModel {
    fn from_payload(payload: &MachOArm64ExecutablePayload) -> Self {
        let entry_point = MachOArm64EntryPoint::Main;
        let text_segment = match payload {
            MachOArm64ExecutablePayload::MainOnly(_) => MachOArm64TextSegment::main_only(),
            MachOArm64ExecutablePayload::MainWithConstData { .. } => {
                MachOArm64TextSegment::main_with_const_section()
            }
        };
        let load_commands = MachOArm64LoadCommands::minimal_main_executable(
            MachOArm64SegmentName::Text,
            entry_point,
        );

        Self {
            entry_point,
            text_segment,
            load_commands,
        }
    }

    pub const fn entry_point(&self) -> MachOArm64EntryPoint {
        self.entry_point
    }

    pub const fn text_segment(&self) -> &MachOArm64TextSegment {
        &self.text_segment
    }

    pub const fn load_commands(&self) -> &MachOArm64LoadCommands {
        &self.load_commands
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64TextSegment {
    const_section: Option<MachOArm64ConstSection>,
}

impl MachOArm64TextSegment {
    const fn main_only() -> Self {
        Self {
            const_section: None,
        }
    }

    const fn main_with_const_section() -> Self {
        Self {
            const_section: Some(MachOArm64ConstSection),
        }
    }

    pub const fn name(&self) -> MachOArm64SegmentName {
        MachOArm64SegmentName::Text
    }

    pub const fn text_section(&self) -> MachOArm64TextSection {
        MachOArm64TextSection
    }

    pub const fn const_section(&self) -> Option<MachOArm64ConstSection> {
        self.const_section
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64TextSection;

impl MachOArm64TextSection {
    pub const fn name(self) -> MachOArm64SectionName {
        MachOArm64SectionName::Text
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64ConstSection;

impl MachOArm64ConstSection {
    pub const fn name(self) -> MachOArm64SectionName {
        MachOArm64SectionName::Const
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOArm64EntryPoint {
    Main,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOArm64SegmentName {
    Text,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOArm64SectionName {
    Text,
    Const,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachOArm64LoadCommands {
    text_segment: MachOArm64Segment64LoadCommand,
    main_entry: MachOArm64MainLoadCommand,
}

impl MachOArm64LoadCommands {
    const fn minimal_main_executable(
        text_segment_name: MachOArm64SegmentName,
        entry_point: MachOArm64EntryPoint,
    ) -> Self {
        Self {
            text_segment: MachOArm64Segment64LoadCommand::new(text_segment_name),
            main_entry: MachOArm64MainLoadCommand::new(entry_point),
        }
    }

    pub const fn text_segment(&self) -> MachOArm64Segment64LoadCommand {
        self.text_segment
    }

    pub const fn main_entry(&self) -> MachOArm64MainLoadCommand {
        self.main_entry
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64Segment64LoadCommand {
    segment_name: MachOArm64SegmentName,
}

impl MachOArm64Segment64LoadCommand {
    const fn new(segment_name: MachOArm64SegmentName) -> Self {
        Self { segment_name }
    }

    pub const fn kind(self) -> MachOArm64LoadCommandKind {
        MachOArm64LoadCommandKind::Segment64
    }

    pub const fn segment_name(self) -> MachOArm64SegmentName {
        self.segment_name
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachOArm64MainLoadCommand {
    entry_point: MachOArm64EntryPoint,
}

impl MachOArm64MainLoadCommand {
    const fn new(entry_point: MachOArm64EntryPoint) -> Self {
        Self { entry_point }
    }

    pub const fn kind(self) -> MachOArm64LoadCommandKind {
        MachOArm64LoadCommandKind::Main
    }

    pub const fn entry_point(self) -> MachOArm64EntryPoint {
        self.entry_point
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachOArm64LoadCommandKind {
    Segment64,
    Main,
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
    model: MachOArm64ExecutableModel,
}

impl MachOArm64ExecutableWriterPlan {
    pub const fn target(&self) -> MachOArm64ExecutableTarget {
        self.target
    }

    pub const fn payload(&self) -> &MachOArm64ExecutablePayload {
        &self.payload
    }

    pub const fn model(&self) -> &MachOArm64ExecutableModel {
        &self.model
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
    let payload = request.payload;
    let model = MachOArm64ExecutableModel::from_payload(&payload);

    MachOArm64ExecutableWriterPlan {
        target: MachOArm64ExecutableTarget::Arm64AppleMacos,
        payload,
        model,
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

    #[test]
    fn plans_public_mach_o_model_for_main_entry_and_text_segment() {
        let main = MachOArm64MainCode::from_emitted_code_bytes([
            0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6,
        ])
        .expect("main code is non-empty");
        let request = MachOArm64ExecutableWriterRequest::main_only(main);

        let plan = plan_mach_o_arm64_executable(request);
        let model = plan.model();

        assert_eq!(model.entry_point(), MachOArm64EntryPoint::Main);
        assert_eq!(model.text_segment().name(), MachOArm64SegmentName::Text);
        assert_eq!(
            model.text_segment().text_section().name(),
            MachOArm64SectionName::Text
        );
        assert_eq!(model.text_segment().const_section(), None);
    }

    #[test]
    fn plans_const_section_only_when_const_payload_is_present() {
        let main = MachOArm64MainCode::from_emitted_code_bytes([
            0x00, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6,
        ])
        .expect("main code is non-empty");
        let const_data = MachOArm64ConstData::from_read_only_section_bytes(*b"hello world\n")
            .expect("const data is non-empty");
        let request = MachOArm64ExecutableWriterRequest::main_with_const_data(main, const_data);

        let plan = plan_mach_o_arm64_executable(request);
        let const_section = plan
            .model()
            .text_segment()
            .const_section()
            .expect("const payload requests __const section");

        assert_eq!(const_section.name(), MachOArm64SectionName::Const);
    }

    #[test]
    fn plans_minimal_load_commands_for_main_executable_model() {
        let main = MachOArm64MainCode::from_emitted_code_bytes([
            0x40, 0x05, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6,
        ])
        .expect("main code is non-empty");
        let request = MachOArm64ExecutableWriterRequest::main_only(main);

        let plan = plan_mach_o_arm64_executable(request);
        let load_commands = plan.model().load_commands();

        assert_eq!(
            load_commands.text_segment().kind(),
            MachOArm64LoadCommandKind::Segment64
        );
        assert_eq!(
            load_commands.text_segment().segment_name(),
            MachOArm64SegmentName::Text
        );
        assert_eq!(
            load_commands.main_entry().kind(),
            MachOArm64LoadCommandKind::Main
        );
        assert_eq!(
            load_commands.main_entry().entry_point(),
            MachOArm64EntryPoint::Main
        );
    }
}
