// src/core/keygen.rs
use crate::core;
use biblatex::Entry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyPart {
    AuthorLastName,
    Year,
    ShortYear,
    TitleFirstWord,
    JournalFirstWord,
}

impl KeyPart {
    pub fn label(&self) -> &str {
        match self {
            Self::AuthorLastName => "Author (Last Name)",
            Self::Year => "Year (Full)",
            Self::ShortYear => "Year (Short)",
            Self::TitleFirstWord => "Title (1st Word)",
            Self::JournalFirstWord => "Journal (1st Word)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyGenConfig {
    pub parts: Vec<KeyPart>,
    pub separator: String,

    // Feature flag for journal abbreviations
    #[serde(default)]
    pub abbreviate_journals: bool,

    // ✅ NEW: Formatting - Indentation Character
    #[serde(default = "default_indent")]
    pub indent_char: char,

    // ✅ NEW: Formatting - Indentation Width
    #[serde(default = "default_indent_width")]
    pub indent_width: u8,

    // ✅ NEW: Formatting - Field Order
    #[serde(default = "default_field_order")]
    pub field_order: Vec<String>,
}

// --- Defaults for Serde ---
fn default_indent() -> char {
    ' '
}
fn default_indent_width() -> u8 {
    4
}
fn default_field_order() -> Vec<String> {
    vec![
        "author".into(),
        "title".into(),
        "year".into(),
        "date".into(),
        "journaltitle".into(),
        "volume".into(),
        "number".into(),
        "pages".into(),
        "doi".into(),
        "url".into(),
    ]
}

impl Default for KeyGenConfig {
    fn default() -> Self {
        Self {
            parts: vec![
                KeyPart::AuthorLastName,
                KeyPart::Year,
                KeyPart::TitleFirstWord,
            ],
            separator: String::new(),
            abbreviate_journals: false,
            // ✅ Initialize new defaults
            indent_char: default_indent(),
            indent_width: default_indent_width(),
            field_order: default_field_order(),
        }
    }
}

pub fn generate_key(entry: &Entry, config: &KeyGenConfig) -> String {
    // ... (Existing implementation remains unchanged) ...
    let mut segments = Vec::new();

    for part in &config.parts {
        let val = match part {
            KeyPart::AuthorLastName => {
                if let Ok(authors) = entry.author() {
                    if let Some(first_author) = authors.iter().next() {
                        first_author.name.clone()
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                }
            }
            KeyPart::Year => entry
                .fields
                .get("year")
                .map(|c| core::bib_to_string(c))
                .unwrap_or_else(|| "0000".to_string()),
            KeyPart::ShortYear => {
                let y = entry
                    .fields
                    .get("year")
                    .map(|c| core::bib_to_string(c))
                    .unwrap_or_else(|| "0000".to_string());
                if y.len() >= 4 {
                    y[2..].to_string()
                } else {
                    y
                }
            }
            KeyPart::TitleFirstWord => entry
                .fields
                .get("title")
                .map(|v| core::bib_to_string(v))
                .map(|t| t.split_whitespace().next().unwrap_or("").to_string())
                .unwrap_or_else(|| "Untitled".to_string()),
            KeyPart::JournalFirstWord => entry
                .fields
                .get("journal")
                .map(|v| core::bib_to_string(v))
                .map(|t| t.split_whitespace().next().unwrap_or("").to_string())
                .unwrap_or_else(|| "Preprint".to_string()),
        };
        let sanitized: String = val.chars().filter(|c: &char| c.is_alphanumeric()).collect();
        segments.push(sanitized);
    }
    segments.join(&config.separator)
}
