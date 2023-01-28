// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;

use iced::futures::TryFutureExt;
use lazy_static::lazy_static;
use nom::{
    branch::{alt, permutation},
    bytes::complete::{tag, take_till, take_until, take_until1},
    character::{
        complete::{anychar, multispace0, newline, not_line_ending, space0},
        is_alphabetic, is_newline,
    },
    combinator::{eof, map, map_res, not, opt, rest, success},
    error::ParseError,
    multi::{many0, many1},
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use regex::Regex;
use std::{any, vec};
use tokio::{fs, io::AsyncBufReadExt};
use tracing::{debug, instrument, trace, warn};

#[derive(Debug, Clone)]
pub enum Token {
    Title(String),
    Section(String),
    Keybind { description: String, keys: String },
    Text(String),
}

const BOUNDARY_TOKEN: &str = "#";
const SECTION_TOKEN: &str = "##";
const HS_COMMENT_SEQ: &str = "--";
const IGNORE_TOKEN: &str = "!";

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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeybindToken<'input>(&'input str, &'input str);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Section<'input> {
    title: Option<&'input str>,
    keybinds: Vec<KeybindToken<'input>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parser<'input> {
    title: Option<&'input str>,
    sections: Vec<Section<'input>>,
}

// impl Parser {
//     fn new() -> Self {
//         Parser::default()
//     }

//     fn parse_boundary(mut self, input: &str) -> IResult<&str, (&str, &str, (), Option<&str>)> {
//         let res = tuple((
//             space0,
//             tag(BOUNDARY_TOKEN),
//             not(tag("#")),
//             opt(take_until("\n")),
//         ))(input);
//         match res {
//             Ok(r) => self.title = r.1 .3.map(String::from),
//             _ => todo!(),
//         }
//         res
//     }
// }

// fn parse_area(input: &str) -> IResult<&str, &str> {
//     let res = tuple((parse_boundary, many0(parse_section), parse_boundary))(input);
//     res
// }

pub fn parse_entry(input: &str) -> IResult<&str, Parser> {
    map(
        ws(tuple((
            parse_boundary,
            many0(alt((
                map(parse_section, Some),
                // map(terminated(not_line_ending, newline), |_| None),
                // map(newline, |_| None),
            ))),
            parse_boundary,
        ))),
        |(title, sections, _)| Parser {
            title,
            sections: sections.into_iter().flatten().collect(),
        },
    )(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn parse_hs_comment_seq(input: &str) -> IResult<&str, ()> {
    map(
        tuple((
            multispace0,
            not(preceded(not(space0), tag(HS_COMMENT_SEQ))),
            tag(HS_COMMENT_SEQ),
            not(tag(">")),
            space0,
        )),
        |(_, _, _, _, _)| (),
    )(input)
}

fn parse_boundary(input: &str) -> IResult<&str, Option<&str>> {
    map(
        tuple((
            parse_hs_comment_seq,
            not(tag(SECTION_TOKEN)),
            tag(BOUNDARY_TOKEN),
            space0,
            opt(terminated(not_line_ending, newline)), // main title
        )),
        |(_, _, _, _, res)| res,
    )(input)
}

fn parse_section_tag(input: &str) -> IResult<&str, Option<&str>> {
    map(
        tuple((
            parse_hs_comment_seq,
            tag(SECTION_TOKEN),
            space0,
            opt(terminated(not_line_ending, newline)), // section title
        )),
        |(_, _, _, title)| title.and_then(|v| if v.is_empty() { None } else { Some(v) }),
    )(input)
}

fn parse_section(input: &str) -> IResult<&str, Section> {
    map(
        ws(tuple((
            parse_section_tag,
            many0(alt((
                map(parse_keybind_declaration, Some),
                map(parse_keybind_comment, Some),
                map(terminated(not_line_ending, newline), |_| None),
            ))),
            parse_section_tag,
        ))),
        |(title, keybinds, _)| Section {
            title,
            keybinds: keybinds.into_iter().flatten().collect(),
        },
    )(input)
}

fn parse_keybind_definition(input: &str) -> IResult<&str, &str> {
    map(
        tuple((
            take_until("("),
            delimited(
                tag("("),
                tuple((
                    space0,
                    delimited(tag("\""), take_until("\""), tag("\"")),
                    take_until(")"),
                )),
                tag(")"),
            ),
        )),
        |(_, (_, key, _))| key,
    )(input)
}

fn parse_keybind_description(input: &str) -> IResult<&str, &str> {
    map(
        tuple((
            parse_hs_comment_seq,
            not(tag(BOUNDARY_TOKEN)),
            not(tag(IGNORE_TOKEN)),
            not(tag("\"")),
            terminated(not_line_ending, newline),
        )),
        |(_, _, _, _, description)| description,
    )(input)
}

fn parse_keybind_declaration(input: &str) -> IResult<&str, KeybindToken> {
    map(
        tuple((parse_keybind_description, parse_keybind_definition)),
        |(d, k)| KeybindToken(k, d),
    )(input)
}

fn parse_keybind_comment(input: &str) -> IResult<&str, KeybindToken> {
    map(
        tuple((
            parse_hs_comment_seq,
            not(tag(BOUNDARY_TOKEN)),
            not(tag(IGNORE_TOKEN)),
            delimited(tag("\""), take_until("\""), tag("\"")), // keymap
            space0,
            terminated(not_line_ending, newline), // description
        )),
        |(_, (), (), k, _, d)| KeybindToken(k, d),
    )(input)
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
            } else if l.starts_with(HS_COMMENT_SEQ) && !l.starts_with(IGNORE_TOKEN) {
                if let Some(value) = strip(l, HS_COMMENT_SEQ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hs_comment_seq_parsing() {
        assert_eq!(parse_hs_comment_seq("--"), Ok(("", ())));
        assert_eq!(parse_hs_comment_seq("--A comment"), Ok(("A comment", ())));
        assert_eq!(parse_hs_comment_seq(" -- A comment"), Ok(("A comment", ())));
        assert!(parse_hs_comment_seq("--> Not a comment").is_err());
        assert!(parse_hs_comment_seq("|-- Not a comment").is_err());
    }

    #[test]
    fn boundary_parsing() {
        assert!(parse_boundary("--").is_err());
        assert!(parse_boundary("-- ").is_err());
        assert!(parse_boundary("-- a").is_err());
        assert_eq!(parse_boundary("-- #"), Ok(("", None)));
        assert_eq!(parse_boundary(" -- #"), Ok(("", None)));
        assert_eq!(parse_boundary(" -- # "), Ok(("", None)));
        assert_eq!(parse_boundary(" -- # Fool"), Ok(("Fool", None)));
        assert_eq!(parse_boundary(" -- # Fool\n"), Ok(("", Some("Fool"))));
        assert_eq!(parse_boundary("--#Fool\n"), Ok(("", Some("Fool"))));
        assert!(parse_boundary("-- ##").is_err());
        assert!(parse_boundary("--##").is_err());
        assert!(parse_boundary("-- ##Fool").is_err());
    }

    #[test]
    fn section_tag_parsing() {
        assert!(parse_section_tag("--").is_err());
        assert!(parse_section_tag("-- ").is_err());
        assert!(parse_section_tag("-- a").is_err());
        assert_eq!(parse_section_tag("--##\n"), Ok(("", None)));
        assert_eq!(parse_section_tag("-- ##\n"), Ok(("", None)));
        assert_eq!(parse_section_tag("-- ##"), Ok(("", None)));
        assert_eq!(parse_section_tag(" -- ##\n"), Ok(("", None)));
        assert_eq!(parse_section_tag(" -- ## \n"), Ok(("", None)));
        assert_eq!(parse_section_tag(" -- ## Fool\n"), Ok(("", Some("Fool"))));
        assert_eq!(parse_section_tag(" -- ## Fool"), Ok(("Fool", None)));
        assert_eq!(parse_section_tag("--##Fool\n"), Ok(("", Some("Fool"))));
        assert_eq!(parse_section_tag("-- ## -- ##\n"), Ok(("", Some("-- ##"))));
        assert!(parse_section_tag("-- #").is_err());
        assert!(parse_section_tag("-- # #").is_err());
        assert!(parse_section_tag("--#").is_err());
        assert!(parse_section_tag("-- #Fool").is_err());
    }

    #[test]
    fn keybind_definition_parsing() {
        assert_eq!(parse_keybind_definition(r#"("M-t")"#), Ok(("", "M-t")));
        assert_eq!(
            parse_keybind_definition(r#", ("M-t", stuff)"#),
            Ok(("", "M-t"))
        );
        assert_eq!(
            parse_keybind_definition(r#", ( "M-)", stuff)"#),
            Ok(("", "M-)"))
        );
        assert_eq!(
            parse_keybind_definition(r#", ( "M-t", stuff)"#),
            Ok(("", "M-t"))
        );
        assert_eq!(
            parse_keybind_definition(
                r#", ( "M-<Space>",
                            stuff)"#
            ),
            Ok(("", "M-<Space>"))
        );
    }

    #[test]
    fn keybind_description_parsing() {
        assert_eq!(parse_keybind_description("--\n"), Ok(("", "")));
        assert_eq!(
            parse_keybind_description("--A description\n"),
            Ok(("", "A description"))
        );
        assert_eq!(
            parse_keybind_description("-- A description\n"),
            Ok(("", "A description"))
        );
        assert_eq!(
            parse_keybind_description(" -- A description\n"),
            Ok(("", "A description"))
        );
        assert!(parse_keybind_description("-- ! Ignored keybind\n").is_err());
        assert!(parse_keybind_description("--! Ignored keybind\n").is_err());
    }

    #[test]
    fn keybind_declaration_parsing() {
        assert_eq!(
            parse_keybind_declaration(
                r#"  -- Recompile and restart XMonad
    ("M-C-q",       spawn "xmonad --recompile; xmonad --restart")"#
            ),
            Ok(("", KeybindToken("M-C-q", "Recompile and restart XMonad")))
        );
        assert_eq!(
            parse_keybind_declaration(
                r#"  -- Recompile and restart XMonad
    ("M-C-q",
        spawn "xmonad --recompile; xmonad --restart")"#
            ),
            Ok(("", KeybindToken("M-C-q", "Recompile and restart XMonad")))
        );
        assert!(parse_keybind_declaration(
            r#"  -- Open a terminal
  , ("M-t,         spawn $ myTerminal)"#
        )
        .is_err());
        assert!(parse_keybind_declaration(r#" , ("M-t", spawn $ myTerminal)"#).is_err());
        assert!(parse_keybind_declaration(
            r#"  -- Open a terminal
              "#
        )
        .is_err());
        assert!(parse_keybind_declaration(
            r#" -- ! Kill current window
                , ("M-x",  kill)"#
        )
        .is_err());
    }

    #[test]
    fn keybind_inline_parsing() {
        assert_eq!(
            parse_keybind_comment("-- \"M-<[]>\" Move to next/previous screen\n"),
            Ok(("", KeybindToken("M-<[]>", "Move to next/previous screen")))
        );
        assert_eq!(
            parse_keybind_comment("--\"M-<[]>\"Move to next/previous screen\n"),
            Ok(("", KeybindToken("M-<[]>", "Move to next/previous screen")))
        );
        assert!(parse_keybind_comment("--# \"M-d\" description\n").is_err());
        assert!(parse_keybind_comment("-- # \"M-d\" description\n").is_err());
        assert!(parse_keybind_comment("-- \"M-d description\n").is_err());
        assert!(parse_keybind_comment("-- M-d description\n").is_err());
        assert!(parse_keybind_comment("-- ! \"M-t\"Open a terminal\n").is_err());
    }

    #[test]
    fn parse_empty_section1() {
        assert!(parse_section(r#" -- ##"#).is_err());
    }

    #[test]
    fn parse_empty_section2() {
        assert!(parse_section(
            r#" -- ##

              "#
        )
        .is_err());
        // assert!(parse_section(r#" -- ## -- ##"#).is_err());
    }

    #[test]
    fn parse_empty_section3() {
        assert!(parse_section(r#" -- ## Section -- ##"#).is_err());
    }

    #[test]
    fn parse_empty_section4() {
        assert_eq!(
            parse_section(
                r#" -- ##
                    -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: None,
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section5() {
        assert_eq!(
            parse_section(
                r#" -- ##
                    -- ## "#
            ),
            Ok((
                "",
                Section {
                    title: None,
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section6() {
        assert_eq!(
            parse_section(
                r#" -- ## Section
                    -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("Section"),
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section7() {
        assert_eq!(
            parse_section(
                r#" -- ## Section

                    -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("Section"),
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section8() {
        assert_eq!(
            parse_section(
                r#" -- ## -- ##
                    -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("-- ##"),
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_section_with_garbage1() {
        assert_eq!(
            parse_section(
                r#"
  -- ## A section
  -- simple comment
  -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("A section"),
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_section_with_garbage2() {
        assert_eq!(
            parse_section(
                r#"
  -- ## A section
  some haskell code
  -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("A section"),
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_section1() {
        assert_eq!(
            parse_section(
                r#"
  -- ## A section
  -- "M-1" desc 1
  -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("A section"),
                    keybinds: vec![KeybindToken("M-1", "desc 1")]
                }
            ))
        );
    }

    #[test]
    fn parse_section2() {
        assert_eq!(
            parse_section(
                r#"
  -- ## A section
  -- "M-1" desc 1
  -- "M-2" desc 2
  -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("A section"),
                    keybinds: vec![KeybindToken("M-1", "desc 1"), KeybindToken("M-2", "desc 2"),]
                }
            ))
        );
    }

    #[test]
    fn parse_section3() {
        assert_eq!(
            parse_section(
                r#"
  -- ## A section
  -- "M-1" desc 1
  -- desc a
  , ("M-a",     spawn "lock.sh")
  -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("A section"),
                    keybinds: vec![KeybindToken("M-1", "desc 1"), KeybindToken("M-a", "desc a"),]
                }
            ))
        );
    }

    #[test]
    fn parse_section4() {
        assert_eq!(
            parse_section(
                r#"
  -- ## A section
  -- "M-1" desc 1
  -- desc a
  , ("M-a",     spawn "lock.sh")
  -- "M-2" desc 2
  -- desc b
  , ("M-b",     sendMessage (IncMasterN 1))
  -- ##"#
            ),
            Ok((
                "",
                Section {
                    title: Some("A section"),
                    keybinds: vec![
                        KeybindToken("M-1", "desc 1"),
                        KeybindToken("M-a", "desc a"),
                        KeybindToken("M-2", "desc 2"),
                        KeybindToken("M-b", "desc b"),
                    ]
                }
            ))
        );
    }

    #[test]
    fn parse_real_case() {
        assert_eq!(
            parse_entry(
                r#"
  -- # Xmonad keymap



  -- ## Section Three
  -- "M-t" desc t
  -- ##

  -- #"#
            ),
            Ok((
                "",
                Parser {
                    title: Some("Xmonad keymap"),
                    sections: vec![
                        Section {
                            title: Some("Section One"),
                            keybinds: vec![
                                KeybindToken("M-1", "desc 1"),
                                KeybindToken("M-a", "desc a"),
                                KeybindToken("M-2", "desc 2"),
                                KeybindToken("M-b", "desc b"),
                            ]
                        },
                        Section {
                            title: Some("Section Two"),
                            keybinds: vec![
                                KeybindToken("M-1", "desc 1"),
                                KeybindToken("M-2", "desc 2"),
                                KeybindToken("M-b", "desc b"),
                            ]
                        },
                        Section {
                            title: Some("Section Three"),
                            keybinds: vec![KeybindToken("M-t", "desc t"),]
                        }
                    ]
                }
            ))
        );
    }
}


  //               r#"
  // -- # Xmonad keymap

  // -- ## Section One
  // -- "M-1" desc 1
  // -- desc a
  // , ("M-a",     spawn "lock.sh")
  // -- "M-2" desc 2
  // -- desc b
  // , ("M-b",     sendMessage (IncMasterN 1))
  // -- ##

  // -- ## Section Two
  // -- "M-1" desc 1
  // -- "M-2" desc 2
  // --! Nope
  // -- desc b
  // , ("M-b",     sendMessage (IncMasterN 1))
  // -- A simple comment
  // -- ##

  // -- a comment
  // some code

  // -- ## Section Three
  // -- "M-t" desc t
  // -- ##

  // -- #"#
