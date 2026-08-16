use vox_proof::candidate::SessionTermEntry;
use vox_proof::session_terms::{SessionTermsError, parse_session_terms};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TermDraft {
    pub canonical: String,
    pub aliases: String,
    pub observed_error_forms: String,
}

impl TermDraft {
    pub fn from_entry(entry: &SessionTermEntry) -> Self {
        Self {
            canonical: entry.canonical_term.clone(),
            aliases: entry.aliases.join(", "),
            observed_error_forms: entry.observed_error_forms.join(", "),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TermsEditor {
    pub terms: Vec<TermDraft>,
}

impl TermsEditor {
    pub fn push_empty_term(&mut self) {
        self.terms.push(TermDraft::default());
    }

    pub fn remove_term(&mut self, index: usize) {
        if index < self.terms.len() {
            self.terms.remove(index);
        }
    }

    pub fn import_from_text(&mut self, text: &str) -> Result<(), SessionTermsError> {
        let entries = parse_session_terms(text)?;
        self.terms = entries.iter().map(TermDraft::from_entry).collect();
        Ok(())
    }

    pub fn to_session_entries(&self) -> Result<Vec<SessionTermEntry>, SessionTermsError> {
        parse_session_terms(&self.to_import_text())
    }

    pub fn to_import_text(&self) -> String {
        self.terms
            .iter()
            .map(|draft| {
                let mut parts = vec![draft.canonical.trim().to_owned()];
                for alias in split_csv(&draft.aliases) {
                    parts.push(format!("alias:{alias}"));
                }
                for observed in split_csv(&draft.observed_error_forms) {
                    parts.push(format!("error:{observed}"));
                }
                parts.join(" | ")
            })
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn split_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_round_trip_through_parser() {
        let mut editor = TermsEditor::default();
        editor.terms.push(TermDraft {
            canonical: "PostgreSQL".to_owned(),
            aliases: "Postgres".to_owned(),
            observed_error_forms: "post gray SQL".to_owned(),
        });
        let entries = editor.to_session_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].canonical_term, "PostgreSQL");
        assert_eq!(entries[0].aliases, vec!["Postgres".to_string()]);
        assert_eq!(
            entries[0].observed_error_forms,
            vec!["post gray SQL".to_string()]
        );
    }

    #[test]
    fn import_from_text_populates_drafts() {
        let mut editor = TermsEditor::default();
        editor
            .import_from_text("Kafka | alias:Kafak\n")
            .unwrap();
        assert_eq!(editor.terms.len(), 1);
        assert_eq!(editor.terms[0].canonical, "Kafka");
    }
}
