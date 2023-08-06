// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fmt;

use iced::{
    alignment::Vertical,
    theme::Palette,
    widget::{column, Row, Text},
    Alignment, Element, Length, Padding,
};
use tracing::{instrument, trace};

use crate::{
    app::{AppConfig, Message},
    parser::Section as ParsedSection,
};

#[derive(Debug, Clone, Default)]
pub struct Keybind {
    pub keys: String,
    pub description: String,
}

impl fmt::Display for Keybind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.keys, self.description)
    }
}

impl Keybind {
    pub fn new(keys: &str, desc: &str) -> Self {
        Keybind {
            keys: keys.to_owned(),
            description: desc.to_owned(),
        }
    }

    fn view(&self, config: &AppConfig, palette: &Palette) -> Element<'static, Message> {
        render_keybind(self.keys.clone(), self.description.clone(), config, palette)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Section {
    pub title: Option<String>,
    pub keybinds: Vec<Keybind>,
}

impl Section {
    #[instrument(skip_all)]
    fn view(&self, config: &AppConfig, palette: &Palette) -> Element<'static, Message> {
        trace!("rendering section {:?}", &self.title);
        let mut content = column![];
        if let Some(t) = &self.title {
            content = content.push(
                Text::new(t.clone())
                    .size(config.ui.section_size)
                    .vertical_alignment(Vertical::Center),
            );
        }

        let keybinds = self.keybinds.iter().fold(column![], |column, keybind| {
            column
                .push(keybind.view(config, palette))
                .width(Length::Fill)
                .spacing(8)
                .padding(Padding::from([12, 0, 0, 12])) // top, right, bottom, left
        });

        content.push(keybinds).into()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tokens {
    pub title: Option<String>,
    pub sections: Vec<Section>,
}

impl Tokens {
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    pub fn keybind_count(&self) -> usize {
        self.sections
            .iter()
            .fold(0, |acc, s| acc + s.keybinds.len())
    }

    // converts all keybinds of all sections into an array of `ScoredKeybind`
    pub fn keybinds(&self) -> Vec<ScoredKeybind> {
        self.sections.iter().fold(vec![], |mut acc, s| {
            acc.append(&mut s.keybinds.iter().map(From::from).collect());
            acc
        })
    }

    #[instrument(skip_all)]
    pub fn view(&self, config: &AppConfig, palette: &Palette) -> Element<'static, Message> {
        trace!("view");
        self.sections
            .iter()
            .fold(column![], |column, section| {
                column.push(section.view(config, palette)).spacing(8)
            })
            .width(Length::Fill)
            .spacing(28)
            .padding(Padding::from([35, 30, 30, 30])) // top, right, bottom, left
            .into()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScoredKeybind {
    pub keys: String,
    pub description: String,
    pub score: Option<(i64, Vec<usize>)>,
}

impl ScoredKeybind {
    pub fn view(&self, config: &AppConfig, palette: &Palette) -> Element<'static, Message> {
        render_keybind(self.keys.clone(), self.description.clone(), config, palette)
    }
}

impl fmt::Display for ScoredKeybind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.keys, self.description)
    }
}

impl From<&Keybind> for ScoredKeybind {
    fn from(keybind: &Keybind) -> Self {
        ScoredKeybind {
            keys: keybind.keys.clone(),
            description: keybind.description.clone(),
            score: None,
        }
    }
}

impl<'input> From<(Option<&str>, Vec<ParsedSection<'input>>)> for Tokens {
    fn from(parsed: (Option<&str>, Vec<ParsedSection<'input>>)) -> Self {
        let sections = parsed
            .1
            .iter()
            .map(|s| Section {
                title: s.title.map(|t| t.to_owned()),
                keybinds: s
                    .keybinds
                    .iter()
                    .map(|token| Keybind::new(token.0, token.1))
                    .collect(),
            })
            .collect();
        Tokens {
            title: parsed.0.map(|t| t.to_owned()),
            sections,
        }
    }
}

fn render_keybind(
    keys: String,
    desc: String,
    config: &AppConfig,
    palette: &Palette,
) -> Element<'static, Message> {
    Row::new()
        .spacing(20)
        .align_items(Alignment::Center)
        .push(
            Text::new(keys)
                .size(config.ui.keybind_size)
                .style(palette.primary),
        )
        .push(Text::new(desc).size(config.ui.text_size))
        .into()
}
