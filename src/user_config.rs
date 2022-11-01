// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use tokio::fs;
use tracing::{debug, error, instrument};

use crate::color::WColor;

// default values
const XMONAD_HS_PATH: &str = "~/.xmonad/xmonad.hs";
pub const BG_COLOR: [u8; 3] = [0x2a, 0x21, 0x1c]; // #2A211C
pub const FG_COLOR: [u8; 3] = [0xbd, 0xae, 0x9d]; // #BDAE9D
pub const KEYBIND_COLOR: [u8; 3] = [0xc5, 0x65, 0x6b]; // #C5656B
pub const SCROLLBAR_COLOR: [u8; 3] = [0x7f, 0x4a, 0x2b]; // #7F4A2B
pub const ERROR_COLOR: [u8; 3] = [0xe5, 0x39, 0x35]; // #e53935
pub const FONT_SIZE: u16 = 20;
pub const TITLE_FONT_SIZE: u16 = 32;

#[derive(Deserialize, Debug)]
pub struct UserConfig {
    pub config_path: String,
    pub colors: Option<Colors>,
    pub font: Option<FontConfig>,
}

#[derive(Deserialize, Debug)]
pub struct Colors {
    pub fg: Option<WColor>,
    pub bg: Option<WColor>,
    pub title: Option<WColor>,
    pub section: Option<WColor>,
    pub keybind: Option<WColor>,
    pub text: Option<WColor>,
    pub scrollbar: Option<WColor>,
    pub error: Option<WColor>,
}

#[derive(Deserialize, Debug)]
pub struct FontConfig {
    pub title_size: Option<u16>,
    pub section_size: Option<u16>,
    pub keybind_size: Option<u16>,
    pub text_size: Option<u16>,
    pub error_size: Option<u16>,
}

impl UserConfig {
    #[instrument]
    pub async fn try_read() -> Result<Self> {
        let home = env::var("HOME").context("Environment variable HOME not set")?;
        let xdg_config_path =
            env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
        let config_path = format!("{}/apekey/apekey.toml", xdg_config_path);
        debug!("user config path {}", config_path);
        let content = fs::read(&config_path).await.context(config_path)?;
        toml::from_slice::<UserConfig>(&content).map_err(|e| {
            error!("{}", e);
            anyhow::Error::new(e)
        })
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        UserConfig {
            config_path: XMONAD_HS_PATH.into(),
            colors: Some(Colors::default()),
            font: Some(FontConfig::default()),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Colors {
            fg: Some(WColor::from(FG_COLOR)),
            bg: Some(WColor::from(BG_COLOR)),
            title: Some(WColor::from(FG_COLOR)),
            section: Some(WColor::from(FG_COLOR)),
            keybind: Some(WColor::from(KEYBIND_COLOR)),
            text: Some(WColor::from(FG_COLOR)),
            scrollbar: Some(WColor::from(SCROLLBAR_COLOR)),
            error: Some(WColor::from(ERROR_COLOR)),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            title_size: Some(TITLE_FONT_SIZE),
            section_size: Some(FONT_SIZE),
            keybind_size: Some(FONT_SIZE),
            text_size: Some(FONT_SIZE),
            error_size: Some(FONT_SIZE),
        }
    }
}
