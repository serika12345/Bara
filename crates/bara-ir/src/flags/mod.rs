#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Flags {
    cf: FlagValue,
    pf: FlagValue,
    af: FlagValue,
    zf: FlagValue,
    sf: FlagValue,
    of: FlagValue,
}

impl Flags {
    pub const fn new(
        cf: FlagValue,
        pf: FlagValue,
        af: FlagValue,
        zf: FlagValue,
        sf: FlagValue,
        of: FlagValue,
    ) -> Self {
        Self {
            cf,
            pf,
            af,
            zf,
            sf,
            of,
        }
    }

    pub const fn unknown() -> Self {
        Self::new(
            FlagValue::Unknown,
            FlagValue::Unknown,
            FlagValue::Unknown,
            FlagValue::Unknown,
            FlagValue::Unknown,
            FlagValue::Unknown,
        )
    }

    pub const fn cf(self) -> FlagValue {
        self.cf
    }

    pub const fn pf(self) -> FlagValue {
        self.pf
    }

    pub const fn af(self) -> FlagValue {
        self.af
    }

    pub const fn zf(self) -> FlagValue {
        self.zf
    }

    pub const fn sf(self) -> FlagValue {
        self.sf
    }

    pub const fn of(self) -> FlagValue {
        self.of
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlagValue {
    Known(bool),
    Unknown,
}

#[cfg(test)]
mod tests {
    use crate::{FlagValue, Flags};

    #[test]
    fn flags_expose_materialized_flag_values() {
        let flags = Flags::new(
            FlagValue::Known(true),
            FlagValue::Known(false),
            FlagValue::Unknown,
            FlagValue::Known(true),
            FlagValue::Known(false),
            FlagValue::Unknown,
        );

        assert_eq!(flags.cf(), FlagValue::Known(true));
        assert_eq!(flags.pf(), FlagValue::Known(false));
        assert_eq!(flags.af(), FlagValue::Unknown);
        assert_eq!(flags.zf(), FlagValue::Known(true));
        assert_eq!(flags.sf(), FlagValue::Known(false));
        assert_eq!(flags.of(), FlagValue::Unknown);
    }

    #[test]
    fn unknown_flags_make_every_tracked_status_unknown() {
        let flags = Flags::unknown();

        assert_eq!(flags.cf(), FlagValue::Unknown);
        assert_eq!(flags.pf(), FlagValue::Unknown);
        assert_eq!(flags.af(), FlagValue::Unknown);
        assert_eq!(flags.zf(), FlagValue::Unknown);
        assert_eq!(flags.sf(), FlagValue::Unknown);
        assert_eq!(flags.of(), FlagValue::Unknown);
    }
}
