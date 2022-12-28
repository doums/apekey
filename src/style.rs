// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use iced::Font;

pub const FONT_MONO: Font = Font::External {
    name: "JetbrainsMono",
    bytes: include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
};

pub const FONT_MEDIUM: Font = Font::External {
    name: "Roboto",
    bytes: include_bytes!("../assets/fonts/Roboto-Medium.ttf"),
};

pub const FONT_BLACK: Font = Font::External {
    name: "Roboto",
    bytes: include_bytes!("../assets/fonts/Roboto-Black.ttf"),
};
