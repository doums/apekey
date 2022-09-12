// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::Parser;
use iced::widget::Text;
use iced::{
    executor, scrollable, Application, Color, Command, Container, Element, Length, Scrollable,
    Settings, Padding,
};
use parser::Token;
#[macro_use]
extern crate log;

mod error;
mod parser;
mod style;

/// xmokey, lists your XMonad keybindings
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Path of your xmonad.hs config file
    #[clap(value_parser)]
    path: String,

    /// Log level, one of ERROR, WARN, INFO, DEBUG, TRACE
    #[clap(short, long)]
    log: Option<log::Level>,

    /// Font size
    #[clap(short, long)]
    font_size: Option<u16>,
}

pub fn main() -> iced::Result {
    env_logger::init();
    let cli = Cli::parse();

    info!("running Xmokey");

    let mut settings = Settings {
        antialiasing: true,
        default_text_size: 22,
        default_font: Some("JetbrainsMono".as_bytes()),
        ..Settings::with_flags(XmokeyFlags {
            config_path: cli.path,
        })
    };
    if let Some(size) = cli.font_size {
        settings.default_text_size = size;
    }

    Xmokey::run(settings)
}

pub struct XmokeyFlags {
    config_path: String,
}

pub struct Xmokey {
    state: State,
    tokens: Vec<Token>,
    scrollable: scrollable::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReaded(Vec<u8>),
    ConfigError(String),
    ParsingDone(Vec<Token>),
    ParsingError(String),
}

impl Application for Xmokey {
    type Executor = executor::Default;
    type Flags = XmokeyFlags;
    type Message = Message;

    fn new(flags: XmokeyFlags) -> (Xmokey, Command<Message>) {
        (
            Xmokey {
                tokens: vec![],
                state: State::ReadingConfig,
                scrollable: scrollable::State::new(),
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
        String::from("Xmokey")
    }

    // fn subscription(&self) -> Subscription<Message> {
    //     iced_native::subscription::events().map(Message::EventOccurred)
    // }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        println!("UPDATE");
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
                println!("received {:#?}", tokens);
                self.tokens = tokens;
                self.state = State::Idle;
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

    fn view(&mut self) -> Element<Self::Message> {
        let red = Color::from_rgb8(168, 32, 32);

        match &self.state {
            State::ReadingConfig => Text::new("... reading xmonad.hs").into(),
            State::ParsingConfig => Text::new("... parsing xmonad.hs").into(),
            State::Idle => self
                .tokens
                .iter()
                .cloned()
                .fold(
                    Scrollable::new(&mut self.scrollable),
                    |scrollable, message| scrollable.push(Text::new("plop")),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(8)
                .padding(Padding::from([20, 30]))
                .style(style::Scrollable)
                .into(),
            State::Error(err) => Text::new(err).color(red).into(),
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

#[derive(Debug)]
enum State {
    ReadingConfig,
    ParsingConfig,
    Idle,
    Error(String),
}
