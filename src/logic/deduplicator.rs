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

    // Collect references to entries so we can index them
    let entries: Vec<_> = bib.iter().collect();
    let n = entries.len();

    if n < 2 {
        return groups;
    }

    // --- OPTIMIZATION (Speed) ---
    // Pre-compute normalized titles ONCE.
    // We removed 'year' from here because strict year checking causes
    // false negatives (e.g., Preprint 2023 vs Paper 2024).
    let normalized_titles: Vec<String> = entries.iter().map(|e| get_title_normalized(e)).collect();
    // ----------------------------

    let mut visited: Vec<bool> = vec![false; n];

    // Pairwise comparison
    for i in 0..n {
        if visited[i] {
            continue;
        }

        let title_a = &normalized_titles[i];

        // Optimization: Skip very short titles to avoid noise (e.g. "Intro", "Data")
        if title_a.len() < 5 {
            continue;
        }

        let mut current_candidates = Vec::new();

        for j in (i + 1)..n {
            if visited[j] {
                continue;
            }

            let title_b = &normalized_titles[j];

            // --- ROBUSTNESS LOGIC ---
            // We removed the year check. Now we ONLY compare titles.
            // This relies on the Jaro-Winkler math being fast enough for N < 2000.
            let similarity = strsim::jaro_winkler(title_a, title_b);

            // Threshold 0.93 allows for small typos or British/American spelling diffs
            if similarity > 0.93 {
                current_candidates.push(DuplicateCandidate {
                    info: extract_info(entries[j]),
                    similarity,
                });
                visited[j] = true;
            }
        }

        // If we found any candidates for this entry, create a group
        if !current_candidates.is_empty() {
            groups.push(DuplicateGroup {
                original: extract_info(entries[i]),
                candidates: current_candidates,
            });
            visited[i] = true; // Mark original as handled
        }
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
