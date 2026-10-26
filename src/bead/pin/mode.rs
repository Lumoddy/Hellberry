#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinMode {
    Ignored,
    DigitalInput,
    DigitalOutput,
    AnalogInput,
    AnalogOutput,
}

impl From<PinMode> for u8 {
    fn from(value: PinMode) -> Self {
        match value {
            PinMode::Ignored => 0,
            PinMode::DigitalInput => 1,
            PinMode::DigitalOutput => 2,
            PinMode::AnalogInput => 3,
            PinMode::AnalogOutput => 4,
        }
    }
}

impl TryFrom<u8> for PinMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ignored),
            1 => Ok(Self::DigitalInput),
            2 => Ok(Self::DigitalOutput),
            3 => Ok(Self::AnalogInput),
            4 => Ok(Self::AnalogOutput),
            _ => Err(()),
        }
    }
}
