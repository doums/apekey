use crate::parser::Parsed;

#[derive(Debug, Clone, Default)]
pub struct KeybindToken(String, String);

#[derive(Debug, Clone, Default)]
pub struct Section {
    pub title: Option<String>,
    pub keybinds: Vec<KeybindToken>,
}

#[derive(Debug, Clone, Default)]
pub struct Tokens {
    pub title: Option<String>,
    pub sections: Vec<Section>,
}

impl Tokens {
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    pub fn keybind_count(&self) -> usize {
        self.sections
            .iter()
            .fold(0, |acc, s| acc + s.keybinds.len())
    }

}

impl<'input> From<Parsed<'input>> for Tokens {
    fn from(parsed: Parsed<'input>) -> Self {
        let sections = parsed
            .sections
            .iter()
            .map(|s| Section {
                title: s.title.map(|t| t.to_owned()),
                keybinds: s
                    .keybinds
                    .iter()
                    .map(|token| KeybindToken(token.0.into(), token.1.into()))
                    .collect(),
            })
            .collect();
        Tokens {
            title: parsed.title.map(|t| t.to_owned()),
            sections,
        }
    }
}
