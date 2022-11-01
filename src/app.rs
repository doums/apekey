use crate::color::WColor;
use crate::parser::{self, Token};
use crate::style::{self, FONT_BLACK, FONT_MEDIUM, FONT_MONO};
use crate::user_config::{
    UserConfig, BG_COLOR, ERROR_COLOR, FG_COLOR, FONT_SIZE, KEYBIND_COLOR, SCROLLBAR_COLOR,
    TITLE_FONT_SIZE,
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
    pub text_color: Color,
    pub scrollbar_color: Color,
    pub error_color: Color,
    pub title_size: u16,
    pub section_size: u16,
    pub keybind_size: u16,
    pub text_size: u16,
    pub error_size: u16,
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
        }
    }

    #[instrument(skip_all)]
    fn view(&mut self) -> Element<Self::Message> {
        match &self.state {
            State::ReadingConfig => Text::new("...").into(),
            State::ParsingConfig => Text::new("...").into(),
            State::RenderKeybinds => {
                let content = self
                    .tokens
                    .iter()
                    .fold(
                        Scrollable::new(&mut self.scrollable).style(style::Scrollable),
                        |scrollable, token| match token {
                            Token::Title(v) => scrollable.push(
                                Text::new(v)
                                    .color(self.config.ui.title_color)
                                    .size(self.config.ui.title_size)
                                    .font(FONT_BLACK)
                                    .height(Length::Units(60)),
                            ),
                            Token::Section(v) => scrollable.push(
                                Text::new(v)
                                    .color(self.config.ui.section_color)
                                    .size(self.config.ui.section_size)
                                    .font(FONT_MEDIUM)
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
                                            .color(self.config.ui.keybind_color)
                                            .size(self.config.ui.keybind_size),
                                    )
                                    .push(Text::new(description).size(self.config.ui.keybind_size)),
                            ),
                            Token::Text(v) => scrollable.push(
                                Text::new(v)
                                    .color(self.config.ui.text_color)
                                    .size(self.config.ui.text_size),
                            ),
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
            State::Error(err) => Text::new(err)
                .color(self.config.ui.error_color)
                .size(self.config.ui.error_size)
                .into(),
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
