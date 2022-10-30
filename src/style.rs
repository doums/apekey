// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use iced::{button, container, scrollable, Color, Font};

pub const FONT_MONO: Font = Font::External {
    name: "JetbrainsMono",
    bytes: include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
};

pub const FONT_REGULAR: Font = Font::External {
    name: "Roboto",
    bytes: include_bytes!("../assets/fonts/Roboto-Regular.ttf"),
};

pub const FONT_MEDIUM: Font = Font::External {
    name: "Roboto",
    bytes: include_bytes!("../assets/fonts/Roboto-Medium.ttf"),
};

pub const FONT_BLACK: Font = Font::External {
    name: "Roboto",
    bytes: include_bytes!("../assets/fonts/Roboto-Black.ttf"),
};

const SURFACE: Color = Color::from_rgb(
    0x33 as f32 / 255.0,
    0x2a as f32 / 255.0,
    0x25 as f32 / 255.0,
);

const ACTIVE: Color = Color::from_rgb(
    0x7F as f32 / 255.0,
    0x4A as f32 / 255.0,
    0x2B as f32 / 255.0,
);

const HOVERED: Color = Color::from_rgb(
    0xB0 as f32 / 255.0,
    0x61 as f32 / 255.0,
    0x33 as f32 / 255.0,
);

pub const BACKGROUND: Color = Color::from_rgb(
    0x2A as f32 / 255.0,
    0x21 as f32 / 255.0,
    0x1C as f32 / 255.0,
);

pub const TEXT: Color = Color::from_rgb(
    0xBD as f32 / 255.0,
    0xAE as f32 / 255.0,
    0x9D as f32 / 255.0,
);

pub struct Container;
pub struct Scrollable;

impl container::StyleSheet for Container {
    fn style(&self) -> container::Style {
        container::Style {
            background: BACKGROUND.into(),
            text_color: TEXT.into(),
            ..container::Style::default()
        }
    }
}

impl scrollable::StyleSheet for Scrollable {
    fn active(&self) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: BACKGROUND.into(),
            border_radius: 2.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: ACTIVE,
                border_radius: 2.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }

    fn hovered(&self) -> scrollable::Scrollbar {
        let active = self.active();

        scrollable::Scrollbar {
            background: Color { a: 0.5, ..SURFACE }.into(),
            scroller: scrollable::Scroller {
                color: HOVERED,
                ..active.scroller
            },
            ..active
        }
    }

    fn dragging(&self) -> scrollable::Scrollbar {
        let hovered = self.hovered();

        scrollable::Scrollbar {
            scroller: scrollable::Scroller {
                color: HOVERED,
                ..hovered.scroller
            },
            ..hovered
        }
    }
}
