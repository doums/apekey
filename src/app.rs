// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::parser::{self, Token};
use crate::user_config::{self, UserConfig, FONT_SIZE, TITLE_FONT_SIZE};

use fuzzy_matcher::skim::SkimMatcherV2;
use iced::alignment::Horizontal;
use iced::futures::TryFutureExt;
use iced::widget::{
    self, column, container, horizontal_rule, scrollable, text, text_input, vertical_space, Column,
    Row, Text,
};
use iced::{
    alignment::Vertical, executor, Alignment, Application, Command, Element, Length, Padding,
};
use iced::{event, keyboard, subscription, Event, Font, Subscription, Theme};
use once_cell::sync::Lazy;
use std::fmt;
use tokio::fs;
use tracing::{debug, error, info, instrument, trace};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);
const DEFAULT_TITLE: &str = "Key bindings";
const SHOW_REGULAR_COMMENT: bool = false;
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
    pub regular_comment: bool,
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

#[derive(Debug, Clone)]
pub struct TokenItem {
    token: Token,
    score: Option<(i64, Vec<usize>)>,
}

impl fmt::Display for TokenItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.token {
            Token::Title(s) | Token::Section(s) | Token::Text(s) => write!(f, "{s}"),
            Token::Keybind { description, keys } => write!(f, "{keys} {description}"),
        }
    }
}

impl From<&Token> for TokenItem {
    fn from(token: &Token) -> Self {
        TokenItem {
            token: token.clone(),
            score: None,
        }
    }
}

pub struct Apekey {
    state: State,
    input_value: String,
    tokens: Vec<Token>,
    filtered_tokens: Vec<TokenItem>,
    config: AppConfig,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigRead(Vec<u8>),
    ConfigError(String),
    ParsingDone(Vec<Token>),
    ParsingError(String),
    InputChanged(String),
    TokensFiltered(Vec<TokenItem>),
    TabPressed { shift: bool },
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Message::ConfigRead(_) => "ConfigRead".into(),
            Message::ConfigError(_) => "ConfigError".into(),
            Message::ParsingDone(tokens) => format!("ParsingDone: {}", tokens.len()),
            Message::ParsingError(_) => "ParsingError".into(),
            Message::InputChanged(input) => format!("InputChanged: {input}"),
            Message::TokensFiltered(tokens) => format!("TokensFiltered: {}", tokens.len()),
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
                filtered_tokens: vec![],
                input_value: "".to_owned(),
                state: State::ReadingConfig,
                config: flags,
            },
            Command::perform(parser::read_config(path), |result| match result {
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

    #[instrument(skip_all)]
    fn update(&mut self, message: Self::Message) -> Command<Message> {
        trace!("{}", message);
        match message {
            Message::ConfigRead(content) => {
                info!("xmonad configuration file was read successfully.");
                self.state = State::ParsingConfig;
                Command::perform(parser::parse(content), |result| match result {
                    Ok(tokens) => Message::ParsingDone(tokens),
                    Err(e) => Message::ParsingError(e.to_string()),
                })
            }
            Message::ParsingDone(tokens) => {
                info!("parsing done, parsed tokens {}", tokens.len());
                self.tokens = tokens;
                self.filtered_tokens = self.tokens.iter().map(TokenItem::from).collect();
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
                Command::perform(filter_tokens(self.tokens.clone(), value), |result| {
                    Message::TokensFiltered(result)
                })
            }
            Message::TokensFiltered(tokens) => {
                info!("fuzzy sorting done, matching tokens {}", tokens.len());
                self.filtered_tokens = tokens;
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
                let text_input = container(
                    text_input("Search", &self.input_value, Message::InputChanged)
                        .id(INPUT_ID.clone())
                        .padding(10)
                        .width(Length::Units(180))
                        .size(20),
                )
                .width(Length::Fill)
                .align_x(Horizontal::Right);

                let title_str = if let Some(Token::Title(v)) =
                    self.tokens.iter().find(|&t| matches!(t, Token::Title(_)))
                {
                    v.to_owned()
                } else {
                    DEFAULT_TITLE.to_owned()
                };
                let title = text(title_str)
                    .size(self.config.ui.title_size)
                    .font(FONT_SS);

                let content = self
                    .filtered_tokens
                    .iter()
                    .fold(Column::new(), |column: _, token| match &token.token {
                        Token::Section(v) => column.push(vertical_space(Length::Units(14))).push(
                            Text::new(v)
                                .size(self.config.ui.section_size)
                                .font(FONT_SS)
                                .vertical_alignment(Vertical::Center),
                        ),
                        Token::Keybind { description, keys } => column.push(
                            Row::new()
                                .spacing(20)
                                .align_items(Alignment::Center)
                                .push(
                                    Text::new(keys)
                                        .font(FONT_MONO)
                                        .size(self.config.ui.keybind_size),
                                )
                                .push(Text::new(description).size(self.config.ui.keybind_size)),
                        ),
                        // TODO fix parser first
                        // Token::Text(v) => {
                        //     if self.config.regular_comment {
                        //         column.push(Text::new(v).size(self.config.ui.text_size))
                        //     } else {
                        //         column
                        //     }
                        // }
                        _ => column,
                    })
                    .width(Length::Fill)
                    .spacing(8)
                    .padding(Padding::from([20, 30]));

                container(column![
                    container(column![title, text_input].spacing(14))
                        .padding(20)
                        .width(Length::Fill),
                    horizontal_rule(1),
                    scrollable(content).height(Length::Fill)
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
async fn filter_tokens(tokens: Vec<Token>, pattern: String) -> Vec<TokenItem> {
    let matcher = SkimMatcherV2::default();
    let mut filtered = tokens.iter().map(TokenItem::from).collect();
    if pattern.is_empty() {
        return filtered;
    }

    filtered = filtered
        .into_iter()
        .map(|mut token| {
            token.score = matcher.fuzzy(&token.to_string(), &pattern, true);
            token
        })
        // only retains keybind tokens with a matching score
        .filter(|token| token.score.is_some() && matches!(&token.token, Token::Keybind { .. }))
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
            regular_comment: config.regular_comment.unwrap_or(SHOW_REGULAR_COMMENT),
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
pub async fn read_config(config_path: String) -> Result<Vec<u8>, Error> {
    fs::read(&config_path)
        .map_err(|e| {
            Error::new(format!(
                "An error occurred while trying to read the config file {}: {}",
                &config_path, e
            ))
        })
        .await
}
