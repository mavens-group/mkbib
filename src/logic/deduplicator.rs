// src/logic/deduplicator.rs
use crate::core;
use biblatex::Bibliography;

/// Holds display-ready metadata for the UI
#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub key: String,
    pub title: String,
    pub author: String,
    pub year: String,
}

/// A specific duplicate match found for an original entry
#[derive(Debug, Clone)]
pub struct DuplicateCandidate {
    pub info: EntryInfo,
    pub similarity: f64, // 0.0 to 1.0 score
}

/// A group containing an Original entry and 1+ Candidates
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    pub original: EntryInfo,
    pub candidates: Vec<DuplicateCandidate>,
}

/// Main logic function: Scans the library for fuzzy duplicates.
/// Returns a list of conflicts to be resolved.
pub fn find_duplicates(bib: &Bibliography) -> Vec<DuplicateGroup> {
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let entries: Vec<_> = bib.iter().collect();
    let mut visited: Vec<bool> = vec![false; entries.len()];

    // Pairwise comparison: O(N^2)
    for i in 0..entries.len() {
        if visited[i] {
            continue;
        }

        let entry_a = entries[i];
        let title_a_norm = get_title_normalized(entry_a);
        let year_a = get_year(entry_a);

        let mut current_candidates = Vec::new();

        for j in (i + 1)..entries.len() {
            if visited[j] {
                continue;
            }

            let entry_b = entries[j];
            let year_b = get_year(entry_b);

            // Optimization 1: Duplicate papers almost always have the same year.
            // This drastically reduces false positives and speeds up scanning.
            if year_a != year_b {
                continue;
            }

            let title_b_norm = get_title_normalized(entry_b);

            // Optimization 2: Jaro-Winkler Similarity
            // Threshold 0.93 allows for small typos or British/American spelling diffs
            // but prevents "Introduction to X" vs "Introduction to Y" false positives.
            let similarity = strsim::jaro_winkler(&title_a_norm, &title_b_norm);

            if similarity > 0.93 {
                current_candidates.push(DuplicateCandidate {
                    info: extract_info(entry_b),
                    similarity,
                });
                visited[j] = true;
            }
        }

        // If we found any candidates for this entry, create a group
        if !current_candidates.is_empty() {
            groups.push(DuplicateGroup {
                original: extract_info(entry_a),
                candidates: current_candidates,
            });
        }

        visited[i] = true;
    }

    groups
}

/// Extracts pretty metadata for display in the dialog
fn extract_info(entry: &biblatex::Entry) -> EntryInfo {
    // Get the raw title (preserve casing for display)
    let title_display = entry
        .fields
        .get("title")
        .map(|c| core::bib_to_string(c))
        .unwrap_or_else(|| "Untitled".to_string());

    // Format authors nicely
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
        .unwrap_or_else(|| "Unknown Author".to_string());

    EntryInfo {
        key: entry.key.clone(),
        title: title_display,
        author: author_display,
        year: get_year(entry),
    }
}

/// Helper: Gets lowercase title for math comparison
fn get_title_normalized(entry: &biblatex::Entry) -> String {
    entry
        .fields
        .get("title")
        .map(|c| core::bib_to_string(c))
        .unwrap_or_default()
        .to_lowercase()
}

/// Helper: Gets year string
fn get_year(entry: &biblatex::Entry) -> String {
    entry
        .fields
        .get("year")
        .map(|c| core::bib_to_string(c))
        .unwrap_or_default()
}
