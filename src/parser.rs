// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use eyre::{eyre, Result};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{multispace0, newline, not_line_ending, space0},
    combinator::{eof, map, not, opt, peek},
    error::ParseError,
    multi::many_till,
    sequence::{delimited, preceded, terminated, tuple},
    Finish, IResult,
};
use tracing::{debug, event, info, instrument, trace, warn};

use crate::{error::Error, token::Tokens};

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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeybindToken<'input>(pub &'input str, pub &'input str);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Section<'input> {
    pub title: Option<&'input str>,
    pub keybinds: Vec<KeybindToken<'input>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parsed<'input> {
    pub title: Option<&'input str>,
    pub sections: Vec<Section<'input>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parser {
    input: String,
}

impl Parser {
    pub fn new(input: String) -> Self {
        Parser { input }
    }

    #[instrument(skip_all)]
    pub async fn parse(self) -> Result<Tokens> {
        info!("start parsing xmonad configuration");
        parse_entry(&self.input)
            .finish()
            .map(|r| Tokens::from(r.1))
            .map_err(|e| eyre!("fail to parse xmonad config: {e}"))
    }
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn parse_inner(input: &str) -> IResult<&str, Option<Section>> {
    ws(alt((
        map(parse_section, Some),
        map(terminated(not_line_ending, newline), |_| None),
    )))(input)
}

#[instrument(skip_all)]
pub fn parse_entry(input: &str) -> IResult<&str, Parsed> {
    map(
        ws(tuple((
            many_till(terminated(not_line_ending, newline), parse_boundary),
            many_till(parse_inner, parse_boundary),
        ))),
        |((_, title), (s, _))| Parsed {
            title,
            sections: s.into_iter().flatten().collect(),
        },
    )(input)
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
        ws(tuple((
            parse_hs_comment_seq,
            not(tag(SECTION_TOKEN)),
            tag(BOUNDARY_TOKEN),
            space0,
            opt(terminated(not_line_ending, newline)), // main title
        ))),
        |(_, _, _, _, title)| title.and_then(|v| if v.is_empty() { None } else { Some(v) }),
    )(input)
}

fn parse_section_tag(input: &str) -> IResult<&str, Option<&str>> {
    map(
        ws(tuple((
            parse_hs_comment_seq,
            tag(SECTION_TOKEN),
            space0,
            opt(terminated(not_line_ending, newline)), // section title
        ))),
        |(_, _, _, title)| title.and_then(|v| if v.is_empty() { None } else { Some(v) }),
    )(input)
}

fn parse_section_inner(input: &str) -> IResult<&str, Option<KeybindToken>> {
    ws(alt((
        map(parse_keybind_declaration, Some),
        map(parse_keybind_comment, Some),
        map(terminated(not_line_ending, newline), |_| None),
    )))(input)
}

fn parse_section(input: &str) -> IResult<&str, Section> {
    map(
        ws(tuple((
            parse_section_tag,
            many_till(
                parse_section_inner,
                alt((
                    map(peek(parse_boundary), |_| ()),
                    map(peek(parse_section_tag), |_| ()),
                    map(eof, |_| ()),
                )),
            ),
        ))),
        |(title, (k, _))| Section {
            title,
            keybinds: k.into_iter().flatten().collect(),
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
    fn boundary_parsing_multilines1() {
        assert_eq!(
            parse_boundary(
                r#"
                -- #   
                "#
            ),
            Ok(("", None))
        );
    }

    #[test]
    fn boundary_parsing_multilines2() {
        assert_eq!(
            parse_boundary(
                r#"
                -- # Title
                "#
            ),
            Ok(("", Some("Title")))
        );
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
    fn section_tag_parsing_multilines1() {
        assert_eq!(
            parse_section_tag(
                r#"
                -- ##   
                "#
            ),
            Ok(("", None))
        );
    }

    #[test]
    fn section_tag_parsing_multilines2() {
        assert_eq!(
            parse_section_tag(
                r#"
                -- ## A Section
                "#
            ),
            Ok(("", Some("A Section")))
        );
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
        assert_eq!(
            parse_section(r#" -- ##"#),
            Ok((
                "",
                Section {
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section2() {
        assert_eq!(
            parse_section(
                r#" -- ##

                "#
            ),
            Ok((
                "",
                Section {
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section3() {
        assert_eq!(
            parse_section(
                r#" -- ## -- ##
                "#
            ),
            Ok((
                "",
                Section {
                    title: Some("-- ##"),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section4() {
        assert_eq!(
            parse_section(
                r#" 
                -- ## Section
                -- ## Another Section"#
            ),
            Ok((
                "-- ## Another Section",
                Section {
                    title: Some("Section"),
                    keybinds: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_empty_section5() {
        assert_eq!(
            parse_section(
                r#" -- ## Section
                "#
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
    fn parse_empty_section6() {
        assert_eq!(
            parse_section(
                r#" -- ## -- ##
                  "#
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
    fn parse_section_stop_on_end_tag() {
        assert_eq!(
            parse_section(
                r#" -- ## Section
                    -- "M-a" desc for A
                    -- #
                    -- "M-b" desc for B
                  "#
            ),
            Ok((
                "-- #\n                    -- \"M-b\" desc for B\n                  ",
                Section {
                    title: Some("Section"),
                    keybinds: vec![KeybindToken("M-a", "desc for A")]
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
  "#
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
  "#
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

  "#
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
  "#
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
  "#
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
  "#
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
    fn parse_simple_case() {
        assert_eq!(
            parse_entry(
                r#"
        some code
        some code
        -- # Xmonad keymap
        -- #
        some code"#
            ),
            Ok((
                "some code",
                Parsed {
                    title: Some("Xmonad keymap"),
                    sections: vec![]
                }
            ))
        );
    }

    #[test]
    fn parse_real_case() {
        assert_eq!(
            parse_entry(
                r#"
        some code...
        -- # Xmonad keymap

        -- ## Section One

        -- "M-1" desc 1
        -- desc a
        , ("M-a",     spawn "lock.sh")

        -- "M-2" desc 2
        -- desc b
        , ("M-b",     sendMessage (IncMasterN 1))

        -- ## Section Two
        -- "M-1" desc 1
        -- "M-2" desc 2
        --! Nope
        , ("M-p",     hi)
        -- desc b
        , ("M-b",     sendMessage (IncMasterN 1))
        -- A simple comment

        -- a comment
        some code
        -- ## Section Three
        some code
        some code
        -- "M-t" desc t
        some code
        some code
        some code

        -- #

        some code...
        "#
            ),
            Ok((
                "some code...\n        ",
                Parsed {
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
