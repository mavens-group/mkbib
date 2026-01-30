// src/logic/action.rs

use biblatex::{Bibliography, Chunk, Entry, Spanned}; // ✅ Added Chunk, Spanned

#[derive(Debug, Clone)]
pub enum Action {
    EntryAdded(Entry),
    EntryDeleted(Entry),
    FieldChanged {
        key: String,
        field: String,
        old_value: String,
        new_value: String,
    },
    Transaction(Vec<Action>),
}

impl Action {
    pub fn apply(&self, bib: &mut Bibliography) {
        match self {
            Action::EntryAdded(entry) => {
                bib.insert(entry.clone());
            }
            Action::EntryDeleted(entry) => {
                bib.remove(&entry.key);
            }
            Action::FieldChanged {
                key,
                field,
                new_value,
                ..
            } => {
                if let Some(entry) = bib.get_mut(key) {
                    // ✅ FIX: Convert String -> Vec<Spanned<Chunk>>
                    let chunks = vec![Spanned::new(Chunk::Normal(new_value.clone()), 0..0)];
                    entry.set(field, chunks);
                }
            }
            Action::Transaction(actions) => {
                for action in actions {
                    action.apply(bib);
                }
            }
        }
    }

    pub fn invert(&self) -> Action {
        match self {
            Action::EntryAdded(entry) => Action::EntryDeleted(entry.clone()),
            Action::EntryDeleted(entry) => Action::EntryAdded(entry.clone()),
            Action::FieldChanged {
                key,
                field,
                old_value,
                new_value,
            } => Action::FieldChanged {
                key: key.clone(),
                field: field.clone(),
                old_value: new_value.clone(),
                new_value: old_value.clone(),
            },
            Action::Transaction(actions) => {
                let mut reversed: Vec<Action> = actions.iter().map(|a| a.invert()).collect();
                reversed.reverse();
                Action::Transaction(reversed)
            }
        }
    }
}
