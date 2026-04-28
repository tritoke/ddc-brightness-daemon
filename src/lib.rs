use std::ops::Neg;

#[derive(Clone, Copy, Debug)]
pub enum BrightnessChange {
    Relative(i16),
    Absolute(u16),
}

impl BrightnessChange {
    pub fn apply(self, value: u16) -> u16 {
        match self {
            Self::Relative(offset) => value.saturating_add_signed(offset),
            Self::Absolute(value) => value,
        }
        .clamp(0, 100)
    }
}

impl std::fmt::Display for BrightnessChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            BrightnessChange::Relative(by) if by < 0 => {
                write!(f, "decrease by {}%", by.clamp(-100, -1).neg())
            }
            BrightnessChange::Relative(by) => write!(f, "increase by {}%", by.clamp(0, 100)),
            BrightnessChange::Absolute(to) => write!(f, "set to {to}%"),
        }
    }
}
