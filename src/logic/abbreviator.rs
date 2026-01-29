// src/logic/abbreviator.rs
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

// --- Tier 1: Exact Match Dictionary ---
fn get_exact_map() -> &'static HashMap<String, String> {
    EXACT_MAP.get_or_init(|| {
        let mut m = HashMap::new();

        let mut loaded_from_disk = false;

        if let Some(dir) = get_data_dir() {
            if !dir.exists() {
                let _ = fs::create_dir_all(&dir);
            }
            let json_path = dir.join("journals.json");

            // If missing, write the embedded default so user can edit it later
            if !json_path.exists() {
                let _ = fs::write(&json_path, DEFAULT_JOURNALS_JSON);
            }

            // Try loading from disk
            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&content) {
                    m.extend(parsed);
                    loaded_from_disk = true;
                }
            }
        }

        // Fallback: Use embedded if disk read failed
        if !loaded_from_disk {
            if let Ok(parsed) =
                serde_json::from_str::<HashMap<String, String>>(DEFAULT_JOURNALS_JSON)
            {
                m.extend(parsed);
            }
        }

        // Normalize keys to lowercase for case-insensitive lookup
        m.into_iter().map(|(k, v)| (k.to_lowercase(), v)).collect()
    })
}

// --- Tier 2: LTWA Word Map ---
fn get_ltwa() -> &'static HashMap<String, String> {
    WORD_MAP.get_or_init(|| {
        let mut m = HashMap::new();

        // 1. Load Starter List
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

        // 2. Load External "ltwa.csv" from Config Dir
        if let Some(proj) = ProjectDirs::from("com", "mkbib", "mkbib-rs") {
            let csv_path = proj.config_dir().join("ltwa.csv");
            if csv_path.exists() {
                // Explicitly type the reader to help the compiler
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

//  helper function to build the reverse map
fn get_reverse_map() -> &'static HashMap<String, String> {
    REVERSE_EXACT_MAP.get_or_init(|| {
        let forward = get_exact_map();
        let mut m = HashMap::new();
        for (full_title, abbr) in forward {
            // Map "j. phys." -> "Journal of Physics"
            m.insert(abbr.to_lowercase(), full_title.clone());
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

    // Tier 1: Exact Match (Case-Insensitive)
    let exact_map = get_exact_map();
    if let Some(abbr) = exact_map.get(&title_clean.to_lowercase()) {
        return abbr.clone();
    }

    // Tier 2: Word-by-Word Abbreviation (ISO 4 style)
    let word_map = get_ltwa();
    let stops = get_stopwords();

    title_clean
        .split_whitespace()
        .filter(|token| {
            // Remove stop words
            let clean = token
                .to_lowercase()
                .trim_matches(|c: char| !c.is_alphabetic())
                .to_string();
            !stops.contains(clean.as_str())
        })
        .map(|token| {
            let clean_word = token
                .to_lowercase()
                .trim_matches(|c: char| !c.is_alphabetic())
                .to_string();

            if let Some(abbr) = word_map.get(&clean_word) {
                abbr.clone()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn unabbreviate_journal(abbr: &str) -> Option<String> {
    let map = get_reverse_map();
    map.get(&abbr.to_lowercase()).cloned()
}
