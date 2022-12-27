use crate::color::WColor;
use crate::parser::{self, Token};
use crate::style::{FONT_BLACK, FONT_MEDIUM, FONT_MONO};
use crate::user_config::{
    UserConfig, BG_COLOR, ERROR_COLOR, FG_COLOR, FONT_SIZE, KEYBIND_COLOR, SCROLLBAR_COLOR,
    TITLE_FONT_SIZE,
};
use iced::widget::{column, container, scrollable, text, text_input, Column, Row, Text};
use iced::{
    alignment::Vertical, executor, Alignment, Application, Color, Command, Element, Length, Padding,
};
use iced::{subscription, Subscription, Theme};
use iced_native::{window, Event};
use tracing::{error, info, instrument};

const DEFAULT_TITLE: &str = "Key bindings";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub config_path: String,
    pub ui: Ui,
}

#[derive(Debug, Clone)]
pub struct Ui {
    pub bg_color: Color,
    pub title_color: Color,
    pub section_color: Color,
    pub keybind_color: Color,
    pub text_color: Color,
    pub scrollbar_color: Color,
    pub error_color: Color,
    pub title_size: u16,
    pub section_size: u16,
    pub keybind_size: u16,
    pub text_size: u16,
    pub error_size: u16,
}

struct TokenItem<'token> {
    token: &'token Token,
    text: &'token str,
    score: Option<(u64, Vec<usize>)>,
}

pub struct Apekey {
    state: State,
    input_value: String,
    tokens: Vec<Token>,
    config: AppConfig,
    should_exit: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReaded(Vec<u8>),
    ConfigError(String),
    ParsingDone(Vec<Token>),
    ParsingError(String),
    InputChanged(String),
    EventOccurred(Event),
}

impl Application for Apekey {
    type Executor = executor::Default;
    type Flags = AppConfig;
    type Message = Message;
    type Theme = Theme;

    fn new(flags: AppConfig) -> (Apekey, Command<Message>) {
        (
            Apekey {
                tokens: vec![],
                input_value: "".to_owned(),
                state: State::ReadingConfig,
                config: flags.clone(),
                should_exit: false,
            },
            Command::perform(
                parser::read_config(flags.config_path),
                |result| match result {
                    Ok(content) => Message::ConfigReaded(content),
                    Err(e) => Message::ConfigError(e.to_string()),
                },
            ),
        )
    }

    fn title(&self) -> String {
        String::from("apekey")
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events().map(Message::EventOccurred)
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    #[instrument(skip_all)]
    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::ConfigReaded(content) => {
                info!("XMonad configuration file was read successfully.");
                self.state = State::ParsingConfig;
                Command::perform(parser::parse(content), |result| match result {
                    Ok(tokens) => Message::ParsingDone(tokens),
                    Err(e) => Message::ParsingError(e.to_string()),
                })
            }
            Message::ParsingDone(tokens) => {
                info!("Parsing done, parsed tokens {}", tokens.len());
                self.tokens = tokens;
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
                self.input_value = value;
                Command::none()
            }
            Message::EventOccurred(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    self.should_exit = true;
                }
                Command::none()
            }
        }
    }

    #[instrument(skip_all)]
    fn view(&self) -> Element<Self::Message> {
        match &self.state {
            State::ReadingConfig => container(Text::new("▪▫▫ Reading xmonad.hs").font(FONT_MONO))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into(),
            State::ParsingConfig => container(Text::new("▪▪▫ Parsing keymap   ").font(FONT_MONO))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into(),
            State::RenderKeybinds => {
                let text_input = text_input("Search", &self.input_value, Message::InputChanged)
                    .padding(10)
                    .width(Length::Units(300))
                    .size(20);

                let title_str = if let Some(Token::Title(v)) =
                    self.tokens.iter().find(|&t| matches!(t, Token::Title(_)))
                {
                    v.to_owned()
                } else {
                    DEFAULT_TITLE.to_owned()
                };
                let title = text(title_str)
                    .size(self.config.ui.title_size)
                    .font(FONT_BLACK)
                    .height(Length::Units(60));

                let content = self
                    .tokens
                    .iter()
                    .fold(Column::new(), |column: _, token| match token {
                        Token::Section(v) => column.push(
                            Text::new(v)
                                .size(self.config.ui.section_size)
                                .font(FONT_MEDIUM)
                                .height(Length::Units(40))
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
                        Token::Text(v) => column.push(Text::new(v).size(self.config.ui.text_size)),
                        _ => column,
                    })
                    .width(Length::Fill)
                    .spacing(8)
                    .padding(Padding::from([20, 30]));

                container(column![
                    container(title).padding(20),
                    container(text_input).padding(20),
                    scrollable(content).height(Length::Fill)
                ])
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into()
            }
            State::Error(err) => Text::new(err).size(self.config.ui.error_size).into(),
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl From<UserConfig> for AppConfig {
    fn from(config: UserConfig) -> Self {
        let colors = config.colors.unwrap_or_default();
        let fg = colors.fg.unwrap_or_else(|| WColor::from(FG_COLOR));
        let bg = colors.bg.unwrap_or_else(|| WColor::from(BG_COLOR));
        let title = colors.title.unwrap_or(fg);
        let section = colors.section.unwrap_or(fg);
        let keybind = colors
            .keybind
            .unwrap_or_else(|| WColor::from(KEYBIND_COLOR));
        let text = colors.text.unwrap_or(fg);
        let scrollbar = colors
            .scrollbar
            .unwrap_or_else(|| WColor::from(SCROLLBAR_COLOR));
        let error = colors.error.unwrap_or_else(|| WColor::from(ERROR_COLOR));
        let font_config = config.font.unwrap_or_default();
        AppConfig {
            config_path: config.config_path,
            ui: Ui {
                bg_color: Color::from(bg),
                title_color: Color::from(title),
                section_color: Color::from(section),
                keybind_color: Color::from(keybind),
                text_color: Color::from(text),
                scrollbar_color: Color::from(scrollbar),
                error_color: Color::from(error),
                title_size: font_config.title_size.unwrap_or(TITLE_FONT_SIZE),
                section_size: font_config.section_size.unwrap_or(FONT_SIZE),
                keybind_size: font_config.keybind_size.unwrap_or(FONT_SIZE),
                text_size: font_config.text_size.unwrap_or(FONT_SIZE),
                error_size: font_config.error_size.unwrap_or(FONT_SIZE),
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
