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
