use bitflags::bitflags;

use super::PinMode;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct PinFlags: u8 {
        const DIGITAL_INPUT = 1 << 0;
        const DIGITAL_OUTPUT = 1 << 1;
        const ANALOG_INPUT = 1 << 2;
        const ANALOG_OUTPUT = 1 << 3;
        const RESERVED = 1 << 4;
    }
}

impl From<PinMode> for PinFlags {
    fn from(value: PinMode) -> Self {
        match value {
            PinMode::Ignored => PinFlags::empty(),
            PinMode::DigitalInput => PinFlags::DIGITAL_INPUT,
            PinMode::DigitalOutput => PinFlags::DIGITAL_OUTPUT,
            PinMode::AnalogInput => PinFlags::ANALOG_INPUT,
            PinMode::AnalogOutput => PinFlags::ANALOG_OUTPUT,
        }
    }
}
