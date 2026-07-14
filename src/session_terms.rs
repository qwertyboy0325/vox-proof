use std::collections::HashMap;
use std::fmt;

use crate::candidate::GlossaryEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTermsError {
    EmptyCanonicalTerm {
        line: usize,
    },
    MissingAlias {
        line: usize,
    },
    EmptyAlias {
        line: usize,
        field: usize,
    },
    DuplicateAlias {
        alias: String,
        first_line: usize,
        duplicate_line: usize,
    },
    DuplicateCanonicalTerm {
        canonical_term: String,
        first_line: usize,
        duplicate_line: usize,
    },
}

impl fmt::Display for SessionTermsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCanonicalTerm { line } => {
                write!(
                    formatter,
                    "invalid session terms at line {line}: canonical term is empty"
                )
            }
            Self::MissingAlias { line } => write!(
                formatter,
                "invalid session terms at line {line}: at least one alias is required"
            ),
            Self::EmptyAlias { line, field } => write!(
                formatter,
                "invalid session terms at line {line}: alias field {field} is empty"
            ),
            Self::DuplicateAlias {
                alias,
                first_line,
                duplicate_line,
            } => write!(
                formatter,
                "invalid session terms: duplicate alias '{alias}' appears on lines {first_line} and {duplicate_line}"
            ),
            Self::DuplicateCanonicalTerm {
                canonical_term,
                first_line,
                duplicate_line,
            } => write!(
                formatter,
                "invalid session terms: duplicate canonical term '{canonical_term}' appears on lines {first_line} and {duplicate_line}"
            ),
        }
    }
}

impl std::error::Error for SessionTermsError {}

/// Parses provisional, session-scoped term input.
///
/// Each non-comment line is `canonical term | alias 1 | alias 2 | ...`.
/// The ASCII pipe is always a delimiter; quoting and escaping are unsupported.
pub fn parse_session_terms(input: &str) -> Result<Vec<GlossaryEntry>, SessionTermsError> {
    let mut entries = Vec::new();
    let mut canonical_lines = HashMap::<String, usize>::new();
    let mut alias_lines = HashMap::<String, usize>::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line = line_index + 1;
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
            continue;
        }

        let fields = raw_line.split('|').map(str::trim).collect::<Vec<_>>();
        let canonical_term = fields[0];
        if canonical_term.is_empty() {
            return Err(SessionTermsError::EmptyCanonicalTerm { line });
        }
        if fields.len() == 1 {
            return Err(SessionTermsError::MissingAlias { line });
        }

        if let Some(first_line) = canonical_lines.get(canonical_term) {
            return Err(SessionTermsError::DuplicateCanonicalTerm {
                canonical_term: canonical_term.to_string(),
                first_line: *first_line,
                duplicate_line: line,
            });
        }

        let mut aliases = Vec::with_capacity(fields.len() - 1);
        for (field_index, alias) in fields[1..].iter().enumerate() {
            if alias.is_empty() {
                return Err(SessionTermsError::EmptyAlias {
                    line,
                    field: field_index + 2,
                });
            }
            if let Some(first_line) = alias_lines.get(*alias) {
                return Err(SessionTermsError::DuplicateAlias {
                    alias: (*alias).to_string(),
                    first_line: *first_line,
                    duplicate_line: line,
                });
            }
            aliases.push((*alias).to_string());
            alias_lines.insert((*alias).to_string(), line);
        }

        canonical_lines.insert(canonical_term.to_string(), line);
        entries.push(GlossaryEntry::new(canonical_term, aliases));
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{SessionTermsError, parse_session_terms};

    #[test]
    fn parses_one_valid_entry() {
        let entries = parse_session_terms("Apache Kafka | Kafka").expect("valid session terms");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].canonical_term, "Apache Kafka");
        assert_eq!(entries[0].aliases, ["Kafka"]);
    }

    #[test]
    fn parses_multiple_aliases() {
        let entries =
            parse_session_terms("Apache Kafka | Kafka | 卡夫卡").expect("valid session terms");

        assert_eq!(entries[0].aliases, ["Kafka", "卡夫卡"]);
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let entries =
            parse_session_terms("  PostgreSQL  |  Postgres  | Postgre SQL \t").expect("valid");

        assert_eq!(entries[0].canonical_term, "PostgreSQL");
        assert_eq!(entries[0].aliases, ["Postgres", "Postgre SQL"]);
    }

    #[test]
    fn ignores_blank_and_comment_lines() {
        let entries = parse_session_terms(
            "\n  # session terms\nApache Kafka | Kafka\n\t\n  # another comment\n",
        )
        .expect("valid session terms");

        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn rejects_empty_canonical_term() {
        assert_eq!(
            parse_session_terms(" | Kafka"),
            Err(SessionTermsError::EmptyCanonicalTerm { line: 1 })
        );
    }

    #[test]
    fn rejects_entry_with_no_aliases() {
        assert_eq!(
            parse_session_terms("Apache Kafka"),
            Err(SessionTermsError::MissingAlias { line: 1 })
        );
    }

    #[test]
    fn rejects_empty_alias_fields() {
        assert_eq!(
            parse_session_terms("Apache Kafka | Kafka | "),
            Err(SessionTermsError::EmptyAlias { line: 1, field: 3 })
        );
    }

    #[test]
    fn rejects_duplicate_aliases_within_one_entry() {
        assert_eq!(
            parse_session_terms("Apache Kafka | Kafka | Kafka"),
            Err(SessionTermsError::DuplicateAlias {
                alias: "Kafka".to_string(),
                first_line: 1,
                duplicate_line: 1,
            })
        );
    }

    #[test]
    fn rejects_duplicate_aliases_across_entries() {
        assert_eq!(
            parse_session_terms("Apache Kafka | Kafka\nAuthor Kafka | Kafka"),
            Err(SessionTermsError::DuplicateAlias {
                alias: "Kafka".to_string(),
                first_line: 1,
                duplicate_line: 2,
            })
        );
    }

    #[test]
    fn rejects_duplicate_canonical_terms() {
        assert_eq!(
            parse_session_terms("PostgreSQL | Postgres\nPostgreSQL | Postgre SQL"),
            Err(SessionTermsError::DuplicateCanonicalTerm {
                canonical_term: "PostgreSQL".to_string(),
                first_line: 1,
                duplicate_line: 2,
            })
        );
    }

    #[test]
    fn reports_physical_source_line_numbers() {
        let error =
            parse_session_terms("# comment\n\nApache Kafka | Kafka\nOther | Kafka").unwrap_err();

        assert_eq!(
            error,
            SessionTermsError::DuplicateAlias {
                alias: "Kafka".to_string(),
                first_line: 3,
                duplicate_line: 4,
            }
        );
    }

    #[test]
    fn preserves_unicode_and_exact_case() {
        let entries =
            parse_session_terms("SHIBUYA SKY | Shibuya Sky | 澀谷 Sky").expect("valid terms");

        assert_eq!(entries[0].canonical_term, "SHIBUYA SKY");
        assert_eq!(entries[0].aliases, ["Shibuya Sky", "澀谷 Sky"]);
    }

    #[test]
    fn does_not_add_canonical_term_as_alias() {
        let entries = parse_session_terms("Apache Kafka | Kafka").expect("valid terms");

        assert_eq!(entries[0].aliases, ["Kafka"]);
        assert!(
            !entries[0]
                .aliases
                .iter()
                .any(|alias| alias == "Apache Kafka")
        );
    }
}
