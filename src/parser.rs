// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;

use iced::futures::TryFutureExt;
use lazy_static::lazy_static;
use regex::Regex;
use std::vec;
use tokio::{fs, io::AsyncBufReadExt};
use tracing::{debug, instrument, trace, warn};

#[derive(Debug, Clone)]
pub enum Token {
    Title(String),
    Section(String),
    Keybind { description: String, keys: String },
    Text(String),
}

const BOUNDARY_TOKEN: &str = "-- #";
const SECTION_TOKEN: &str = "-- ##";
const KEYBIND_TOKEN: &str = "-- ";

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

fn strip<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    if let Some(v) = line.strip_prefix(prefix) {
        if !v.trim().is_empty() {
            return Some(v.trim());
        }
    }
    None
}

fn parse_inline_keybind(line: &str) -> Option<(String, String)> {
    let re: Regex = Regex::new(r#"^"(.+?)"(.+)"#).unwrap();
    let caps = re.captures(line);
    if let Some(c) = caps {
        let keys = c.get(1).map(|c| String::from(c.as_str().trim()));
        let description = c.get(2).map(|c| String::from(c.as_str().trim()));
        if let Some(k) = keys {
            if let Some(d) = description {
                return Some((k, d));
            }
        }
    }
    None
}

fn parse_keybind(line: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#""(.+?)""#).unwrap();
    }
    let mat = RE.find(line);
    if let Some(m) = mat {
        return Some(String::from(&line[m.start() + 1..m.end() - 1]));
    }
    None
}

#[instrument(skip_all)]
pub async fn parse(buf: Vec<u8>) -> Result<Vec<Token>, Error> {
    let mut tokens = vec![];
    let mut start_found = false;
    let mut lines = buf.lines();

    while let Some(line) = lines.next_line().await? {
        let l = line.trim();
        if !start_found && l.starts_with(BOUNDARY_TOKEN) {
            debug!("start token found");
            start_found = true;
            if let Some(value) = strip(l, BOUNDARY_TOKEN) {
                tokens.push(Token::Title(value.to_owned()));
            }
        } else if start_found {
            if l.starts_with(BOUNDARY_TOKEN) && l.ends_with(BOUNDARY_TOKEN) {
                debug!("end token found");
                return Ok(tokens);
            }
            if l.starts_with(SECTION_TOKEN) {
                if let Some(value) = strip(l, SECTION_TOKEN) {
                    trace!("section token found [{}]", &value);
                    tokens.push(Token::Section(value.to_owned()));
                }
            } else if l.starts_with(KEYBIND_TOKEN) {
                if let Some(value) = strip(l, KEYBIND_TOKEN) {
                    if let Some((keys, description)) = parse_inline_keybind(value) {
                        let t = Token::Keybind { description, keys };
                        trace!("keybind token found {:#?}", &t);
                        tokens.push(t)
                    } else if let Some(next_line) = lines.next_line().await? {
                        if let Some(k) = parse_keybind(&next_line) {
                            let t = Token::Keybind {
                                description: value.to_owned(),
                                keys: k,
                            };
                            trace!("keybind token found {:#?}", &t);
                            tokens.push(t)
                        } else {
                            trace!("text token found [{}]", value);
                            tokens.push(Token::Text(value.to_owned()));
                        }
                    } else {
                        trace!("text token found [{}]", value);
                        tokens.push(Token::Text(value.to_owned()));
                    }
                }
            }
        }
    }

    warn!("The parsing ended without the end token");
    Ok(tokens)
}
