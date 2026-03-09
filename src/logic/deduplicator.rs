// src/logic/deduplicator.rs
//
// Duplicate detection strategy:
//
//   1. AUTHOR MATCH (primary gate) — normalized last-name sets must overlap
//      significantly. If authors share zero last names, entries are never
//      considered duplicates regardless of title similarity.
//
//   2. TITLE SIMILARITY — Jaro-Winkler on normalized titles, but the
//      threshold adapts: short titles (< 40 chars) require higher similarity
//      to avoid false positives like "Spin Transport" vs "Spin Dynamics".
//
//   3. YEAR PROXIMITY — same or ±1 year gives a small bonus. Mismatch by
//      2+ years is a penalty but not a veto (preprint 2023 → paper 2024).
//
//   Combined score = 0.50 * author_sim + 0.35 * title_sim + 0.15 * year_sim
//   Minimum combined score to flag: 0.80

use crate::core;
use biblatex::Bibliography;

/// Holds display-ready metadata for the UI
#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub key: String,
    pub title: String,
    pub author: String,
    pub year: String,
    pub journal: String,
}

/// A specific duplicate match found for an original entry
#[derive(Debug, Clone)]
pub struct DuplicateCandidate {
    pub info: EntryInfo,
    pub similarity: f64, // 0.0 to 1.0 combined score
}

/// A group containing an Original entry and 1+ Candidates
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    pub original: EntryInfo,
    pub candidates: Vec<DuplicateCandidate>,
}

// Pre-computed data for each entry to avoid repeated parsing
struct EntryData {
    author_lastnames: Vec<String>, // normalized, sorted last names
    title_normalized: String,      // lowercase, whitespace-collapsed
    year: Option<i32>,             // parsed year, if available
}

/// Main logic function: Scans the library for fuzzy duplicates.
/// Returns a list of conflicts to be resolved.
pub fn find_duplicates(bib: &Bibliography) -> Vec<DuplicateGroup> {
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    let entries: Vec<_> = bib.iter().collect();
    let n = entries.len();

    if n < 2 {
        return groups;
    }

    // Pre-compute normalized data for all entries
    let data: Vec<EntryData> = entries.iter().map(|e| precompute(e)).collect();

    let mut visited: Vec<bool> = vec![false; n];

    for i in 0..n {
        if visited[i] {
            continue;
        }

        // Skip entries with no meaningful title
        if data[i].title_normalized.len() < 10 {
            continue;
        }

        let mut current_candidates = Vec::new();

        for j in (i + 1)..n {
            if visited[j] {
                continue;
            }

            if data[j].title_normalized.len() < 10 {
                continue;
            }

            // --- STAGE 1: Author gate ---
            // If both entries have authors, they must share at least one last name.
            // If neither has authors, skip the gate (allow title-only comparison).
            let author_sim = compute_author_similarity(&data[i], &data[j]);
            let both_have_authors =
                !data[i].author_lastnames.is_empty() && !data[j].author_lastnames.is_empty();

            if both_have_authors && author_sim < 0.01 {
                // Zero author overlap with known authors → not a duplicate
                continue;
            }

            // --- STAGE 2: Title similarity ---
            let title_sim =
                strsim::jaro_winkler(&data[i].title_normalized, &data[j].title_normalized);

            // Adaptive threshold: short titles need higher similarity
            let title_len = data[i]
                .title_normalized
                .len()
                .min(data[j].title_normalized.len());
            let title_threshold = if title_len < 30 {
                0.97 // Very short titles: almost exact match required
            } else if title_len < 60 {
                0.94 // Medium titles
            } else {
                0.90 // Long titles: more room for typos/spelling variations
            };

            if title_sim < title_threshold {
                continue;
            }

            // --- STAGE 3: Year proximity ---
            let year_sim = compute_year_similarity(&data[i], &data[j]);

            // --- COMBINE ---
            let combined = if both_have_authors {
                0.50 * author_sim + 0.35 * title_sim + 0.15 * year_sim
            } else {
                // No authors to compare — rely more heavily on title
                0.10 * author_sim + 0.75 * title_sim + 0.15 * year_sim
            };

            if combined >= 0.80 {
                current_candidates.push(DuplicateCandidate {
                    info: extract_info(entries[j]),
                    similarity: combined,
                });
                visited[j] = true;
            }
        }

        if !current_candidates.is_empty() {
            // Sort candidates by similarity descending
            current_candidates.sort_by(|a, b| {
                b.similarity
                    .partial_cmp(&a.similarity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            groups.push(DuplicateGroup {
                original: extract_info(entries[i]),
                candidates: current_candidates,
            });
            visited[i] = true;
        }
    }

    groups
}

// ---------------------------------------------------------------------------
// Pre-computation
// ---------------------------------------------------------------------------

fn precompute(entry: &biblatex::Entry) -> EntryData {
    EntryData {
        author_lastnames: get_author_lastnames(entry),
        title_normalized: get_title_normalized(entry),
        year: get_year_parsed(entry),
    }
}

/// Extract normalized, sorted last names from the author field.
fn get_author_lastnames(entry: &biblatex::Entry) -> Vec<String> {
    entry
        .author()
        .ok()
        .map(|authors| {
            let mut names: Vec<String> = authors
                .iter()
                .map(|p| {
                    p.name
                        .to_lowercase()
                        .chars()
                        .filter(|c| c.is_alphabetic() || c.is_whitespace())
                        .collect::<String>()
                        .trim()
                        .to_string()
                })
                .filter(|n| !n.is_empty())
                .collect();
            names.sort();
            names
        })
        .unwrap_or_default()
}

/// Lowercase, whitespace-collapsed, stripped of punctuation for comparison.
fn get_title_normalized(entry: &biblatex::Entry) -> String {
    entry
        .fields
        .get("title")
        .map(|c| core::bib_to_string(c))
        .unwrap_or_default()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn get_year_parsed(entry: &biblatex::Entry) -> Option<i32> {
    entry
        .fields
        .get("year")
        .map(|c| core::bib_to_string(c))
        .and_then(|s| s.trim().parse::<i32>().ok())
}

fn get_year_display(entry: &biblatex::Entry) -> String {
    entry
        .fields
        .get("year")
        .map(|c| core::bib_to_string(c))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Similarity functions
// ---------------------------------------------------------------------------

/// Computes author similarity based on last-name set overlap.
///
/// Uses Jaccard-like similarity: |intersection| / |smaller set|
/// This way, if entry A has authors [Smith, Jones, Lee] and entry B has
/// [Smith, Jones], the similarity is 2/2 = 1.0 (B's authors are a subset).
///
/// Fuzzy matching on individual names handles typos (Müller vs Mueller).
fn compute_author_similarity(a: &EntryData, b: &EntryData) -> f64 {
    if a.author_lastnames.is_empty() || b.author_lastnames.is_empty() {
        // If either has no authors, return a neutral score
        return 0.5;
    }

    let smaller = a.author_lastnames.len().min(b.author_lastnames.len());
    if smaller == 0 {
        return 0.0;
    }

    // Count how many names in the smaller set have a fuzzy match in the larger set
    let (short, long) = if a.author_lastnames.len() <= b.author_lastnames.len() {
        (&a.author_lastnames, &b.author_lastnames)
    } else {
        (&b.author_lastnames, &a.author_lastnames)
    };

    let mut matches = 0;
    let mut used: Vec<bool> = vec![false; long.len()];

    for s_name in short {
        let mut best_score = 0.0;
        let mut best_idx = None;

        for (idx, l_name) in long.iter().enumerate() {
            if used[idx] {
                continue;
            }
            let sim = strsim::jaro_winkler(s_name, l_name);
            if sim > best_score {
                best_score = sim;
                best_idx = Some(idx);
            }
        }

        // Require >= 0.85 similarity for a name match (handles Müller/Mueller)
        if best_score >= 0.85 {
            if let Some(idx) = best_idx {
                used[idx] = true;
                matches += 1;
            }
        }
    }

    matches as f64 / smaller as f64
}

/// Year similarity:
///   Same year → 1.0
///   ±1 year   → 0.8  (preprint vs published)
///   ±2 years  → 0.4
///   Farther   → 0.0
///   Missing   → 0.5  (neutral, don't penalize missing data)
fn compute_year_similarity(a: &EntryData, b: &EntryData) -> f64 {
    match (a.year, b.year) {
        (Some(ya), Some(yb)) => {
            let diff = (ya - yb).unsigned_abs();
            match diff {
                0 => 1.0,
                1 => 0.8,
                2 => 0.4,
                _ => 0.0,
            }
        }
        _ => 0.5, // Missing year → neutral
    }
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

/// Extracts pretty metadata for display in the dialog
fn extract_info(entry: &biblatex::Entry) -> EntryInfo {
    let title_display = entry
        .fields
        .get("title")
        .map(|c| core::bib_to_string(c))
        .unwrap_or_else(|| "Untitled".to_string())
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");

    let author_display = entry
        .author()
        .ok()
        .map(|authors| {
            authors
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "Unknown Author".to_string())
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");

    // Try journaltitle first (biblatex), then journal (bibtex), then archiveprefix/eprint
    let journal_display = entry
        .fields
        .get("journaltitle")
        .or_else(|| entry.fields.get("journal"))
        .map(|c| core::bib_to_string(c))
        .map(|s| s.split_whitespace().collect::<Vec<&str>>().join(" "))
        .unwrap_or_else(|| {
            // Check for arXiv or other preprint identifiers
            if entry.fields.contains_key("eprint") {
                let prefix = entry
                    .fields
                    .get("archiveprefix")
                    .map(|c| core::bib_to_string(c))
                    .unwrap_or_else(|| "Preprint".to_string());
                prefix
            } else {
                String::new()
            }
        });

    EntryInfo {
        key: entry.key.clone(),
        title: title_display,
        author: author_display,
        year: get_year_display(entry),
        journal: journal_display,
    }
}
