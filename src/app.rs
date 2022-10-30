use crate::parser::{self, Token};
use crate::style::{self, FONT_BLACK, FONT_MEDIUM, FONT_MONO};
use crate::user_config::{
    UserConfig, BG_COLOR, ERROR_COLOR, FG_COLOR, KEYBIND_COLOR, SCROLLBAR_COLOR,
};
use iced::widget::Text;
use iced::{
    alignment::Vertical, executor, scrollable, Alignment, Application, Color, Command, Container,
    Element, Length, Padding, Row, Scrollable,
};
use tracing::{error, info, instrument};

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
    pub comment_color: Color,
    pub scrollbar_color: Color,
    pub error_color: Color,
}

pub struct Apekey {
    state: State,
    tokens: Vec<Token>,
    scrollable: scrollable::State,
    config: AppConfig,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReaded(Vec<u8>),
    ConfigError(String),
    ParsingDone(Vec<Token>),
    ParsingError(String),
}

impl Application for Apekey {
    type Executor = executor::Default;
    type Flags = AppConfig;
    type Message = Message;

    fn new(flags: AppConfig) -> (Apekey, Command<Message>) {
        (
            Apekey {
                tokens: vec![],
                state: State::ReadingConfig,
                scrollable: scrollable::State::new(),
                config: flags.clone(),
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

    // fn subscription(&self) -> Subscription<Message> {
    //     iced_native::subscription::events().map(Message::EventOccurred)
    // }

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
                info!("parsing done, parsed tokens {}", tokens.len());
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
        }
    }

    #[instrument(skip_all)]
    fn view(&mut self) -> Element<Self::Message> {
        match &self.state {
            State::ReadingConfig => Text::new("...").into(),
            State::ParsingConfig => Text::new("...").into(),
            State::RenderKeybinds => {
                let content =
                    self.tokens
                        .iter()
                        .fold(
                            Scrollable::new(&mut self.scrollable).style(style::Scrollable),
                            |scrollable, token| match token {
                                Token::Title(v) => scrollable.push(
                                    Text::new(v)
                                        .color(self.config.ui.title_color)
                                        .size(40)
                                        .font(FONT_BLACK)
                                        .height(Length::Units(60)),
                                ),
                                Token::Section(v) => scrollable.push(
                                    Text::new(v)
                                        .color(self.config.ui.section_color)
                                        .font(FONT_MEDIUM)
                                        .size(20)
                                        .height(Length::Units(40))
                                        .vertical_alignment(Vertical::Center),
                                ),
                                Token::Keybind { description, keys } => scrollable.push(
                                    Row::new()
                                        .spacing(20)
                                        .align_items(Alignment::Center)
                                        .push(
                                            Text::new(keys)
                                                .font(FONT_MONO)
                                                .color(self.config.ui.keybind_color),
                                        )
                                        .push(Text::new(description)),
                                ),
                                Token::Text(v) => scrollable
                                    .push(Text::new(v).color(self.config.ui.comment_color)),
                            },
                        )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .spacing(8)
                        .padding(Padding::from([20, 30]))
                        .style(style::Scrollable);
                Container::new(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(style::Container)
                    .into()
            }
            State::Error(err) => Text::new(err).color(self.config.ui.error_color).into(),
        }

        // Container::new(Text::new("TEST"))
        //     .width(Length::Fill)
        //     .height(Length::Fill)
        //     .center_x()
        //     .center_y()
        //     .style(style::Container)
        //     .into()
    }
}

impl From<UserConfig> for AppConfig {
    fn from(config: UserConfig) -> Self {
        let colors = config.colors.unwrap_or_default();
        let bg = colors.bg.unwrap_or_else(|| BG_COLOR.into());
        let title = colors.title.unwrap_or_else(|| FG_COLOR.into());
        let section = colors.section.unwrap_or_else(|| FG_COLOR.into());
        let keybind = colors.keybind.unwrap_or_else(|| KEYBIND_COLOR.into());
        let comment = colors.comment.unwrap_or_else(|| FG_COLOR.into());
        let scrollbar = colors.scrollbar.unwrap_or_else(|| SCROLLBAR_COLOR.into());
        let error = colors.error.unwrap_or_else(|| ERROR_COLOR.into());
        AppConfig {
            config_path: config.config_path,
            ui: Ui {
                bg_color: Color::from_rgb8(bg.r, bg.g, bg.b),
                title_color: Color::from_rgb8(title.r, title.g, title.b),
                section_color: Color::from_rgb8(section.r, section.g, section.b),
                keybind_color: Color::from_rgb8(keybind.r, keybind.g, keybind.b),
                comment_color: Color::from_rgb8(comment.r, comment.g, comment.b),
                scrollbar_color: Color::from_rgb8(scrollbar.r, scrollbar.g, scrollbar.b),
                error_color: Color::from_rgb8(error.r, error.g, error.b),
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
