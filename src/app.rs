// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::parser::Parser;
use crate::token::{ScoredKeybind, Tokens};
use crate::user_config::{self, UserConfig, FONT_SIZE, TITLE_FONT_SIZE};

use eyre::{eyre, Result};
use fuzzy_matcher::skim::SkimMatcherV2;
use iced::alignment::Horizontal;
use iced::futures::TryFutureExt;
use iced::widget::{self, column, container, horizontal_rule, scrollable, text, text_input, Text};
use iced::{event, keyboard, subscription, Event, Font, Subscription, Theme};
use iced::{executor, Application, Command, Element, Length, Padding};
use once_cell::sync::{Lazy, OnceCell};
use std::fmt;
use tokio::fs;
use tracing::{debug, error, info, instrument, trace};

// tokens parsed from xmonad config declared as static as it will
// not change during the whole app lifetime
static TOKENS: OnceCell<Tokens> = OnceCell::new();

static FUZZY_MATCHER: Lazy<SkimMatcherV2> = Lazy::new(SkimMatcherV2::default);
static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);
const DEFAULT_TITLE: &str = "Key bindings";
// Monospace font
pub const FONT_MONO: Font = Font::External {
    name: "JetbrainsMono",
    bytes: include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
};
// Sans Serif font
pub const FONT_SS: Font = Font::External {
    name: "Roboto",
    bytes: include_bytes!("../assets/fonts/Roboto-Regular.ttf"),
};

#[derive(Debug)]
pub struct AppConfig {
    pub config_path: String,
    pub ui: Ui,
    pub theme: Theme,
}

#[derive(Debug, Clone)]
pub struct Ui {
    pub title_size: u16,
    pub section_size: u16,
    pub keybind_size: u16,
    pub text_size: u16,
    pub error_size: u16,
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            title_size: TITLE_FONT_SIZE,
            section_size: FONT_SIZE,
            keybind_size: FONT_SIZE,
            text_size: FONT_SIZE,
            error_size: FONT_SIZE,
        }
    }
}

pub struct Apekey {
    state: State,
    input_value: String,
    // this field is used to store the matching keybinds when fuzzy
    // searching
    tokens: Vec<ScoredKeybind>,
    config: AppConfig,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigRead(String),
    ConfigError(String),
    ParsingDone(Tokens),
    ParsingError(String),
    InputChanged(String),
    TokensFiltered(Vec<ScoredKeybind>),
    TabPressed { shift: bool },
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Message::ConfigRead(_) => "ConfigRead".into(),
            Message::ConfigError(_) => "ConfigError".into(),
            Message::ParsingDone(_) => "ParsingDone".into(),
            Message::ParsingError(_) => "ParsingError".into(),
            Message::InputChanged(input) => format!("InputChanged: {input}"),
            Message::TokensFiltered(_) => "TokensFiltered".into(),
            Message::TabPressed { shift } => format!("TabPressed, shift {shift}"),
        };
        write!(f, "{message}")
    }
}

impl Application for Apekey {
    type Executor = executor::Default;
    type Flags = AppConfig;
    type Message = Message;
    type Theme = Theme;

    fn new(flags: AppConfig) -> (Apekey, Command<Message>) {
        let path = flags.config_path.clone();
        (
            Apekey {
                tokens: vec![],
                input_value: "".to_owned(),
                state: State::ReadingConfig,
                config: flags,
            },
            Command::perform(read_config(path), |result| match result {
                Ok(content) => Message::ConfigRead(content),
                Err(e) => Message::ConfigError(e.to_string()),
            }),
        )
    }

    fn title(&self) -> String {
        String::from("apekey")
    }

    #[instrument(skip_all)]
    fn subscription(&self) -> Subscription<Message> {
        subscription::events_with(|event, status| match (event, status) {
            (
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Tab,
                    modifiers,
                    ..
                }),
                event::Status::Ignored,
            ) => Some(Message::TabPressed {
                shift: modifiers.shift(),
            }),
            _ => None,
        })
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        trace!("{}", message);
        match message {
            Message::ConfigRead(config) => {
                info!("xmonad configuration file was read successfully.");
                self.state = State::ParsingConfig;
                Command::perform(parse(config), |result| match result {
                    Ok(tokens) => Message::ParsingDone(tokens),
                    Err(e) => Message::ParsingError(e.to_string()),
                })
            }
            Message::ParsingDone(tokens) => {
                TOKENS.set(tokens).unwrap();
                let tokens = TOKENS.get().unwrap();
                info!(
                    "parsing done, sections {}, keybinds {}",
                    tokens.section_count(),
                    tokens.keybind_count()
                );
                self.state = State::RenderKeybinds;
                Command::none()
            }
            Message::ConfigError(err) => {
                error!("{}", err);
                self.state = State::Error(err);
                Command::none()
            }
            Message::ParsingError(err) => {
                error!("{}", err);
                self.state = State::Error(err);
                Command::none()
            }
            Message::InputChanged(value) => {
                self.input_value = value.clone();
                if value.is_empty() {
                    Command::none()
                } else {
                    Command::perform(
                        filter_tokens(
                            TOKENS.get().expect("TOKENS not initialized!").keybinds(),
                            value,
                        ),
                        |tokens| -> Message { Message::TokensFiltered(tokens) },
                    )
                }
            }
            Message::TokensFiltered(tokens) => {
                self.tokens = tokens;
                info!("fuzzy sorting done, matching tokens {}", self.tokens.len());
                Command::none()
            }
            Message::TabPressed { shift } => {
                if shift {
                    debug!("message: focus prev");
                    widget::focus_previous()
                } else {
                    debug!("message: focus next");
                    widget::focus_next()
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn view(&self) -> Element<Self::Message> {
        match &self.state {
            State::ReadingConfig => container(Text::new("▪▫▫ Reading xmonad.hs").font(FONT_MONO))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20)
                .center_x()
                .center_y()
                .into(),
            State::ParsingConfig => container(Text::new("▪▪▫ Parsing keymap   ").font(FONT_MONO))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20)
                .center_x()
                .center_y()
                .into(),
            State::RenderKeybinds => {
                debug!("rendering keybinds");
                let tokens = TOKENS.get().unwrap();
                let text_input = container(
                    text_input("Search", &self.input_value, Message::InputChanged)
                        .id(INPUT_ID.clone())
                        .padding(10)
                        .width(Length::Units(180))
                        .size(20),
                )
                .width(Length::Fill)
                .align_x(Horizontal::Right);

                let default_title = DEFAULT_TITLE.to_string();
                let title = text(tokens.title.as_ref().unwrap_or(&default_title))
                    .size(self.config.ui.title_size)
                    .font(FONT_SS);

                let keybinds = if self.input_value.is_empty() {
                    scrollable(tokens.view(&self.config))
                } else {
                    scrollable(self.tokens.iter().fold(column![], |column, keybind| {
                        column
                            .push(keybind.view(&self.config))
                            .width(Length::Fill)
                            .spacing(8)
                            .padding(Padding::from([35, 30, 30, 30])) // top, right, bottom, left
                    }))
                };

                container(column![
                    container(column![title, text_input].spacing(14))
                        .padding(20)
                        .width(Length::Fill),
                    horizontal_rule(1),
                    keybinds.height(Length::Fill)
                ])
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into()
            }
            State::Error(err) => container(
                Text::new(err)
                    .size(self.config.ui.error_size)
                    .width(Length::Units(400)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .center_x()
            .center_y()
            .into(),
        }
    }

    fn theme(&self) -> Theme {
        self.config.theme.clone()
    }
}

#[instrument(skip_all)]
async fn filter_tokens(mut tokens: Vec<ScoredKeybind>, pattern: String) -> Vec<ScoredKeybind> {
    for token in &mut tokens {
        token.score = FUZZY_MATCHER.fuzzy(&token.to_string(), &pattern, true);
    }

    let mut filtered: Vec<ScoredKeybind> = tokens
        .into_iter()
        // only retains keybind tokens with a matching score
        .filter(|token| token.score.is_some())
        .collect();

    // sort by fuzzy score
    filtered.sort_by(|a, b| {
        b.score
            .as_ref()
            .unwrap()
            .0
            .cmp(&a.score.as_ref().unwrap().0)
    });
    filtered
}

async fn parse(config: String) -> Result<Tokens> {
    let parser = Parser(config);
    parser.parse().await
}

impl From<UserConfig> for AppConfig {
    fn from(config: UserConfig) -> Self {
        let font_config = config.font.unwrap_or_default();
        AppConfig {
            config_path: config.xmonad_config,
            theme: config
                .theme
                .map(|t| match t {
                    user_config::Theme::Dark => Theme::Dark,
                    user_config::Theme::Light => Theme::Light,
                })
                .unwrap_or_else(|| Theme::Dark),
            ui: Ui {
                title_size: font_config.title_size.unwrap_or_default(),
                section_size: font_config.section_size.unwrap_or_default(),
                keybind_size: font_config.keybind_size.unwrap_or_default(),
                text_size: font_config.text_size.unwrap_or_default(),
                error_size: font_config.error_size.unwrap_or_default(),
            },
        }
    }
}

#[derive(Debug)]
enum State {
    ReadingConfig,
    ParsingConfig,
    RenderKeybinds,
    Error(String),
}

#[instrument]
pub async fn read_config(config_path: String) -> Result<String> {
    fs::read_to_string(&config_path)
        .map_err(|e| eyre!("Failed to read the config file {config_path}\n{e}"))
        .await
}
