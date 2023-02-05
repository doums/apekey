// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod app;
mod parser;
mod token;
mod user_config;

use crate::{
    app::{Apekey, AppConfig},
    user_config::UserConfig,
};
use clap::Parser;
use dotenv::dotenv;
use iced::{Application, Settings};
use std::env;
use tracing::{info, trace, warn, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// apekey, lists your XMonad keymap
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Path of your xmonad.hs config file
    #[clap(value_parser)]
    path: Option<String>,

    /// Log level, one of trace, debug, info, warn, error
    #[clap(short, long)]
    log: Option<tracing::Level>,

    /// Font size
    #[clap(short, long)]
    font_size: Option<u16>,
}

fn main() -> iced::Result {
    dotenv().ok();
    let cli = Cli::parse();

    // Tracing init
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| format!("apekey={}", cli.log.unwrap_or(Level::INFO))),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut user_config = UserConfig::try_read().unwrap_or_else(|e| {
        warn!("Failed to read user config: {}", e);
        warn!("Fallback to default config");
        UserConfig::default()
    });
    trace!("User config: {:#?}", &user_config);

    // Override xmonad.hs path if provided as CLI argument
    if let Some(p) = cli.path {
        user_config.xmonad_config = p;
    }
    info!("Path to XMonad config file: {}", &user_config.xmonad_config);

    let mut settings = Settings {
        default_text_size: 22,
        default_font: Some(include_bytes!("../assets/fonts/Roboto-Regular.ttf")),
        ..Settings::with_flags(AppConfig::from(user_config))
    };
    if let Some(size) = cli.font_size {
        settings.default_text_size = size;
    }

    info!("Starting apekey");
    Apekey::run(settings)
}
