use serde::{Deserialize, Serialize};

/// A color struct that holds the red, green, and blue values of a color.
#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

static WHITE: Color = Color {
    red: 255,
    green: 255,
    blue: 255,
};

impl Default for Color {
    fn default() -> Self {
        Self::white()
    }
}

impl Color {
    pub fn white() -> Self {
        WHITE
    }

    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub fn r(&self) -> u8 {
        self.red
    }

    pub fn g(&self) -> u8 {
        self.green
    }

    pub fn b(&self) -> u8 {
        self.blue
    }
}

#[cfg(feature = "bevy")]
impl From<Color> for bevy::prelude::Color {
    fn from(color: Color) -> Self {
        bevy::prelude::Color::srgb_u8(color.red, color.green, color.blue)
    }
}
