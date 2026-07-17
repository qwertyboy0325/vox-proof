use std::collections::HashMap;
use std::fmt;

use crate::candidate::SessionTermEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTermsError {
    EmptyCanonicalTerm {
        line: usize,
    },
    /// Legacy compatibility variant. Canonical-only entries are valid and current
    /// parsing/validation paths do not emit this error solely because source-form
    /// collections are empty.
    MissingSourceForm {
        line: usize,
    },
    UnknownPrefix {
        line: usize,
        field: usize,
        prefix: String,
    },
    UnprefixedSourceForm {
        line: usize,
        field: usize,
    },
    EmptyAlias {
        line: usize,
        field: usize,
    },
    EmptyObservedErrorForm {
        line: usize,
        field: usize,
    },
    DuplicateAlias {
        alias: String,
        first_line: usize,
        duplicate_line: usize,
    },
    DuplicateObservedErrorForm {
        observed_error_form: String,
        first_line: usize,
        duplicate_line: usize,
    },
    ConflictingSourceFormKinds {
        source_form: String,
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
            Self::MissingSourceForm { line } => write!(
                formatter,
                "invalid session terms at line {line}: at least one prefixed alias or observed error form is required"
            ),
            Self::UnknownPrefix {
                line,
                field,
                prefix,
            } => write!(
                formatter,
                "invalid session terms at line {line}: field {field} has unsupported prefix '{prefix}'"
            ),
            Self::UnprefixedSourceForm { line, field } => write!(
                formatter,
                "invalid session terms at line {line}: field {field} must use alias: or error:"
            ),
            Self::EmptyAlias { line, field } => write!(
                formatter,
                "invalid session terms at line {line}: alias field {field} is empty"
            ),
            Self::EmptyObservedErrorForm { line, field } => write!(
                formatter,
                "invalid session terms at line {line}: observed error form field {field} is empty"
            ),
            Self::DuplicateAlias {
                alias,
                first_line,
                duplicate_line,
            } => write!(
                formatter,
                "invalid session terms: duplicate alias '{alias}' appears on lines {first_line} and {duplicate_line}"
            ),
            Self::DuplicateObservedErrorForm {
                observed_error_form,
                first_line,
                duplicate_line,
            } => write!(
                formatter,
                "invalid session terms: duplicate observed error form '{observed_error_form}' appears on lines {first_line} and {duplicate_line}"
            ),
            Self::ConflictingSourceFormKinds {
                source_form,
                first_line,
                duplicate_line,
            } => write!(
                formatter,
                "invalid session terms: source form '{source_form}' is both an alias and observed error form on lines {first_line} and {duplicate_line}"
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
/// Each non-comment line is either `canonical term` or
/// `canonical term | alias:alternate form | error:observed ASR form | ...`.
/// The ASCII pipe is always a delimiter; quoting and escaping are unsupported.
pub fn parse_session_terms(input: &str) -> Result<Vec<SessionTermEntry>, SessionTermsError> {
    let mut entries = Vec::new();
    let mut canonical_lines = HashMap::<String, usize>::new();
    let mut source_form_lines = HashMap::<String, (SourceFormKind, usize)>::new();

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

        if let Some(first_line) = canonical_lines.get(canonical_term) {
            return Err(SessionTermsError::DuplicateCanonicalTerm {
                canonical_term: canonical_term.to_string(),
                first_line: *first_line,
                duplicate_line: line,
            });
        }

        let mut aliases = Vec::new();
        let mut observed_error_forms = Vec::new();
        for (field_index, field) in fields[1..].iter().enumerate() {
            let field_number = field_index + 2;
            let (kind, value) = parse_source_form_field(field, line, field_number)?;

            if let Some((first_kind, first_line)) = source_form_lines.get(value) {
                let error = match (first_kind, kind) {
                    (SourceFormKind::Alias, SourceFormKind::Alias) => {
                        SessionTermsError::DuplicateAlias {
                            alias: value.to_string(),
                            first_line: *first_line,
                            duplicate_line: line,
                        }
                    }
                    (SourceFormKind::ObservedError, SourceFormKind::ObservedError) => {
                        SessionTermsError::DuplicateObservedErrorForm {
                            observed_error_form: value.to_string(),
                            first_line: *first_line,
                            duplicate_line: line,
                        }
                    }
                    _ => SessionTermsError::ConflictingSourceFormKinds {
                        source_form: value.to_string(),
                        first_line: *first_line,
                        duplicate_line: line,
                    },
                };
                return Err(error);
            }

            match kind {
                SourceFormKind::Alias => aliases.push(value.to_string()),
                SourceFormKind::ObservedError => observed_error_forms.push(value.to_string()),
            }
            source_form_lines.insert(value.to_string(), (kind, line));
        }

        canonical_lines.insert(canonical_term.to_string(), line);
        entries.push(SessionTermEntry::new(
            canonical_term,
            aliases,
            observed_error_forms,
        ));
    }

    Ok(entries)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFormKind {
    Alias,
    ObservedError,
}

fn parse_source_form_field(
    field: &str,
    line: usize,
    field_number: usize,
) -> Result<(SourceFormKind, &str), SessionTermsError> {
    let Some((prefix, raw_value)) = field.split_once(':') else {
        return Err(SessionTermsError::UnprefixedSourceForm {
            line,
            field: field_number,
        });
    };
    let value = raw_value.trim();

    match prefix.trim() {
        "alias" if value.is_empty() => Err(SessionTermsError::EmptyAlias {
            line,
            field: field_number,
        }),
        "alias" => Ok((SourceFormKind::Alias, value)),
        "error" if value.is_empty() => Err(SessionTermsError::EmptyObservedErrorForm {
            line,
            field: field_number,
        }),
        "error" => Ok((SourceFormKind::ObservedError, value)),
        unsupported => Err(SessionTermsError::UnknownPrefix {
            line,
            field: field_number,
            prefix: unsupported.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionTermsError, parse_session_terms};

    #[test]
    fn parses_valid_alias_only_entry() {
        let entries =
            parse_session_terms("Apache Kafka | alias:Kafka").expect("valid session terms");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].canonical_term, "Apache Kafka");
        assert_eq!(entries[0].aliases, ["Kafka"]);
        assert!(entries[0].observed_error_forms.is_empty());
    }

    #[test]
    fn parses_valid_observed_error_only_entry() {
        let entries =
            parse_session_terms("Apache Kafka | error:阿帕契卡夫卡").expect("valid session terms");

        assert!(entries[0].aliases.is_empty());
        assert_eq!(entries[0].observed_error_forms, ["阿帕契卡夫卡"]);
    }

    #[test]
    fn parses_mixed_alias_and_observed_error_entry() {
        let entries = parse_session_terms("SHIBUYA SKY | alias:Shibuya Sky | error:澀谷 Sky")
            .expect("valid session terms");

        assert_eq!(entries[0].aliases, ["Shibuya Sky"]);
        assert_eq!(entries[0].observed_error_forms, ["澀谷 Sky"]);
    }

    #[test]
    fn canonical_only_entry_does_not_emit_missing_source_form() {
        match parse_session_terms("ASUS") {
            Ok(entries) => {
                assert_eq!(entries.len(), 1);
                assert!(entries[0].aliases.is_empty());
                assert!(entries[0].observed_error_forms.is_empty());
            }
            Err(SessionTermsError::MissingSourceForm { .. }) => {
                panic!("canonical-only entry must not emit MissingSourceForm");
            }
            Err(other) => panic!("unexpected parse error: {other}"),
        }
    }

    #[test]
    fn legacy_missing_source_form_variant_remains_available_for_compatibility() {
        let first = SessionTermsError::MissingSourceForm { line: 3 };
        let second = SessionTermsError::MissingSourceForm { line: 3 };

        assert_eq!(first, second);
        assert_eq!(
            first.to_string(),
            "invalid session terms at line 3: at least one prefixed alias or observed error form is required"
        );
        assert_eq!(format!("{first:?}"), "MissingSourceForm { line: 3 }");
    }

    #[test]
    fn valid_canonical_only_paths_never_emit_missing_source_form() {
        for input in [
            "ASUS",
            "Google Translate",
            "華碩",
            "ASUS\nApache Kafka | alias:Kafka",
        ] {
            match parse_session_terms(input) {
                Ok(_) => {}
                Err(SessionTermsError::MissingSourceForm { line }) => {
                    panic!("input {input:?} must not emit MissingSourceForm at line {line}");
                }
                Err(other) => panic!("input {input:?} failed unexpectedly: {other}"),
            }
        }
    }

    #[test]
    fn canonical_only_pipeline_does_not_emit_missing_source_form() {
        use crate::pipeline::run_term_review;
        use crate::srt::parse_srt;

        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nASIS").expect("valid srt");
        let entries = parse_session_terms("ASUS").expect("canonical-only parse");

        assert!(run_term_review(&transcript, &entries).is_ok());
    }

    #[test]
    fn parses_canonical_only_ascii_entry_with_empty_source_forms() {
        let entries = parse_session_terms("ASUS").expect("valid canonical-only term");

        assert_eq!(
            entries,
            [crate::candidate::SessionTermEntry::new(
                "ASUS",
                Vec::new(),
                Vec::new()
            )]
        );
    }

    #[test]
    fn parses_multi_word_canonical_only_entry() {
        let entries =
            parse_session_terms("Google Translate").expect("valid multi-word canonical term");

        assert_eq!(entries[0].canonical_term, "Google Translate");
        assert!(entries[0].aliases.is_empty());
        assert!(entries[0].observed_error_forms.is_empty());
    }

    #[test]
    fn parses_unicode_canonical_only_entry() {
        let entries = parse_session_terms("華碩").expect("valid Unicode canonical term");

        assert_eq!(entries[0].canonical_term, "華碩");
        assert!(entries[0].aliases.is_empty());
        assert!(entries[0].observed_error_forms.is_empty());
    }

    #[test]
    fn parses_mixed_entry_kinds_in_source_order() {
        let entries = parse_session_terms(
            "ASUS\nApache Kafka | alias:Kafka\nPostgreSQL | error:post gray sequel",
        )
        .expect("valid mixed session terms");

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.canonical_term.as_str())
                .collect::<Vec<_>>(),
            ["ASUS", "Apache Kafka", "PostgreSQL"]
        );
        assert!(entries[0].aliases.is_empty());
        assert_eq!(entries[1].aliases, ["Kafka"]);
        assert_eq!(entries[2].observed_error_forms, ["post gray sequel"]);
    }

    #[test]
    fn legacy_self_referential_alias_remains_explicit_and_parseable() {
        let entries = parse_session_terms("ASUS | alias:ASUS").expect("valid legacy entry");

        assert_eq!(entries[0].canonical_term, "ASUS");
        assert_eq!(entries[0].aliases, ["ASUS"]);
        assert!(entries[0].observed_error_forms.is_empty());
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let entries =
            parse_session_terms("  PostgreSQL  |  alias:  Postgres  | error:  Postgre SQL \t")
                .expect("valid");

        assert_eq!(entries[0].canonical_term, "PostgreSQL");
        assert_eq!(entries[0].aliases, ["Postgres"]);
        assert_eq!(entries[0].observed_error_forms, ["Postgre SQL"]);
    }

    #[test]
    fn ignores_blank_and_comment_lines() {
        let entries = parse_session_terms(
            "\n  # session terms\nApache Kafka | alias:Kafka\n\t\n  # another comment\n",
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
        assert_eq!(
            parse_session_terms("\t | alias:Kafka"),
            Err(SessionTermsError::EmptyCanonicalTerm { line: 1 })
        );
    }

    #[test]
    fn rejects_unprefixed_source_form() {
        assert_eq!(
            parse_session_terms("Apache Kafka | Kafka"),
            Err(SessionTermsError::UnprefixedSourceForm { line: 1, field: 2 })
        );
    }

    #[test]
    fn rejects_unknown_prefix() {
        assert_eq!(
            parse_session_terms("Apache Kafka | typo:Kafka"),
            Err(SessionTermsError::UnknownPrefix {
                line: 1,
                field: 2,
                prefix: "typo".to_string(),
            })
        );
    }

    #[test]
    fn rejects_empty_alias_value() {
        assert_eq!(
            parse_session_terms("Apache Kafka | alias: "),
            Err(SessionTermsError::EmptyAlias { line: 1, field: 2 })
        );
    }

    #[test]
    fn rejects_empty_observed_error_value() {
        assert_eq!(
            parse_session_terms("Apache Kafka | error: "),
            Err(SessionTermsError::EmptyObservedErrorForm { line: 1, field: 2 })
        );
    }

    #[test]
    fn rejects_duplicate_aliases_within_one_entry() {
        assert_eq!(
            parse_session_terms("Apache Kafka | alias:Kafka | alias:Kafka"),
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
            parse_session_terms("Apache Kafka | alias:Kafka\nAuthor Kafka | alias:Kafka",),
            Err(SessionTermsError::DuplicateAlias {
                alias: "Kafka".to_string(),
                first_line: 1,
                duplicate_line: 2,
            })
        );
    }

    #[test]
    fn rejects_duplicate_observed_error_forms() {
        assert_eq!(
            parse_session_terms("Apache Kafka | error:卡夫卡\nAuthor Kafka | error:卡夫卡",),
            Err(SessionTermsError::DuplicateObservedErrorForm {
                observed_error_form: "卡夫卡".to_string(),
                first_line: 1,
                duplicate_line: 2,
            })
        );
    }

    #[test]
    fn rejects_same_form_as_alias_and_observed_error() {
        assert_eq!(
            parse_session_terms("Apache Kafka | alias:Kafka\nOther | error:Kafka",),
            Err(SessionTermsError::ConflictingSourceFormKinds {
                source_form: "Kafka".to_string(),
                first_line: 1,
                duplicate_line: 2,
            })
        );
    }

    #[test]
    fn rejects_same_source_form_mapping_to_different_canonical_terms() {
        assert_eq!(
            parse_session_terms("Apache Kafka | alias:Kafka\nAuthor Kafka | alias:Kafka",),
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
            parse_session_terms("PostgreSQL | alias:Postgres\nPostgreSQL | error:Postgre SQL",),
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
            parse_session_terms("# comment\n\nApache Kafka | alias:Kafka\nOther | error:Kafka")
                .unwrap_err();

        assert_eq!(
            error,
            SessionTermsError::ConflictingSourceFormKinds {
                source_form: "Kafka".to_string(),
                first_line: 3,
                duplicate_line: 4,
            }
        );
    }

    #[test]
    fn preserves_unicode_and_exact_case() {
        let entries = parse_session_terms("SHIBUYA SKY | alias:Shibuya Sky | error:澀谷 Sky")
            .expect("valid terms");

        assert_eq!(entries[0].canonical_term, "SHIBUYA SKY");
        assert_eq!(entries[0].aliases, ["Shibuya Sky"]);
        assert_eq!(entries[0].observed_error_forms, ["澀谷 Sky"]);
    }

    #[test]
    fn does_not_add_canonical_term_as_alias() {
        let entries = parse_session_terms("Apache Kafka | alias:Kafka").expect("valid terms");

        assert_eq!(entries[0].aliases, ["Kafka"]);
        assert!(
            !entries[0]
                .aliases
                .iter()
                .any(|alias| alias == "Apache Kafka")
        );
    }
}
