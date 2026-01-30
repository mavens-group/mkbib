// src/logic/abbreviator.rs

use crate::core;
use directories::ProjectDirs;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

// 1. Embedded Defaults (Fallback)
const DEFAULT_JOURNALS_JSON: &str = include_str!("../../journals.json");
const STARTER_LTWA: &str = "
Word,Abbreviation
Journal,J.
American,Am.
Chemical,Chem.
Society,Soc.
Physics,Phys.
Review,Rev.
Letters,Lett.
Nature,Nature
Science,Science
Communications,Commun.
Advanced,Adv.
Materials,Mater.
International,Int.
Research,Res.
Engineering,Eng.
Biomedical,Biomed.
Transactions,Trans.
";

static STOPWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
static EXACT_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
static WORD_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
static REVERSE_EXACT_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();

fn get_data_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "mkbib", "mkbib-rs").map(|proj| proj.data_dir().to_path_buf())
}

fn get_stopwords() -> &'static HashSet<&'static str> {
    STOPWORDS.get_or_init(|| {
        HashSet::from([
            "of", "the", "and", "in", "for", "on", "to", "with", "a", "an", "at", "by", "from",
        ])
    })
}

// ✅ NEW HELPER: Convert "physical review b" -> "Physical Review B"
fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                // Capitalize first letter, keep the rest as is
                Some(f) => f.to_uppercase().collect::<String>() + &word[1..],
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

// --- Tier 1: Exact Match Dictionary ---
fn get_exact_map() -> &'static HashMap<String, String> {
    EXACT_MAP.get_or_init(|| {
        let mut m = HashMap::new();
        let mut loaded = false;

        // 1. Try Loading from Disk
        if let Some(dir) = get_data_dir() {
            if !dir.exists() {
                let _ = fs::create_dir_all(&dir);
            }

            let json_path = dir.join("journals.json");

            if !json_path.exists() {
                let _ = fs::write(&json_path, DEFAULT_JOURNALS_JSON);
            }

            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&content) {
                    for (k, v) in parsed {
                        m.insert(k.to_lowercase(), v);
                    }
                    loaded = true;
                }
            }
        }

        // 2. Fallback to Embedded if Disk Failed
        if !loaded {
            let parsed: HashMap<String, String> =
                serde_json::from_str(DEFAULT_JOURNALS_JSON).unwrap_or_default();
            for (k, v) in parsed {
                m.insert(k.to_lowercase(), v);
            }
        }

        m
    })
}

// --- Tier 2: LTWA Word Map ---
fn get_ltwa() -> &'static HashMap<String, String> {
    WORD_MAP.get_or_init(|| {
        let mut m = HashMap::new();

        // 1. Load Starter List (Embedded)
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(STARTER_LTWA.as_bytes());

        for result in rdr.records() {
            if let Ok(record) = result {
                if let (Some(w), Some(a)) = (record.get(0), record.get(1)) {
                    m.insert(w.to_lowercase(), a.to_string());
                }
            }
        }

        // 2. Load External "ltwa.csv" (User Overrides)
        if let Some(dir) = get_data_dir() {
            let csv_path = dir.join("ltwa.csv");
            if csv_path.exists() {
                if let Ok(mut file_rdr) = csv::Reader::from_path(csv_path) {
                    for result in file_rdr.records() {
                        if let Ok(record) = result {
                            let word = record.get(0).unwrap_or("").to_lowercase();
                            let abbr = record.get(1).unwrap_or("").to_string();
                            if !word.is_empty() {
                                m.insert(word, abbr);
                            }
                        }
                    }
                }
            }
        }
        m
    })
}

// --- Helper: Reverse Map (Abbr -> Full) ---
fn get_reverse_map() -> &'static HashMap<String, String> {
    REVERSE_EXACT_MAP.get_or_init(|| {
        let forward = get_exact_map();
        let mut m = HashMap::new();

        for (full_title, abbr) in forward {
            // STRICT MAPPING
            m.insert(abbr.to_lowercase(), full_title.clone());

            // FUZZY MAPPING (via core::normalize)
            m.insert(core::normalize(abbr), full_title.clone());
        }
        m
    })
}

/// The Main Public API
/// Full to Abbreviation
pub fn abbreviate_journal(title: &str) -> String {
    let title_clean = title.trim();
    if title_clean.is_empty() {
        return String::new();
    }

    // Tier 1: Exact Match
    let exact_map = get_exact_map();
    if let Some(abbr) = exact_map.get(&title_clean.to_lowercase()) {
        return abbr.clone();
    }

    // Tier 2: Word-by-Word Abbreviation
    let word_map = get_ltwa();
    let stops = get_stopwords();
    let mut result = String::with_capacity(title.len());
    let mut first = true;

    for token in title_clean.split_whitespace() {
        let clean_word = token
            .to_lowercase()
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_string();

        if stops.contains(clean_word.as_str()) {
            continue;
        }

        if !first {
            result.push(' ');
        }

        if let Some(abbr) = word_map.get(&clean_word) {
            result.push_str(abbr);
        } else {
            result.push_str(token);
        }

        first = false;
    }

    result
}

/// Reverse Lookup: "Phys. Rev. B" -> "Physical Review B"
/// Handles "phys rev b" (no dots) via normalize_key
pub fn unabbreviate_journal(abbr: &str) -> Option<String> {
    let map = get_reverse_map();

    // 1. Try exact match first
    if let Some(full) = map.get(&abbr.to_lowercase()) {
        return Some(to_title_case(full)); // ✅ Fix: Force Title Case
    }

    // 2. Try normalized match
    if let Some(full) = map.get(&core::normalize(abbr)) {
        return Some(to_title_case(full)); // ✅ Fix: Force Title Case
    }

    None
}
