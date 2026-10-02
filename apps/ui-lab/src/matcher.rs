//! Near-sound matching for remembered spellings.
//!
//! Reuses `vox_proof::experimental_retrieval` (non-authoritative) to decide
//! whether a flagged place sounds like a term the user already confirmed, even
//! when the speech recogniser spelled it differently this time.

use crate::app::pages::MemEntry;
use crate::data::Lecture;
use vox_proof::candidate::SessionTermEntry;
use vox_proof::experimental_retrieval::{
    ExperimentalRetrievalConfig, retrieve_experimental_candidates,
};
use vox_proof::srt::parse_srt;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hit {
    pub entry: usize,
    /// True when the flagged text equals the remembered misheard form.
    pub exact: bool,
}

#[derive(Default)]
pub struct Analysis {
    /// Best enabled entry per flagged place.
    pub per_issue: Vec<Option<Hit>>,
    /// Every place each entry would match, whether or not it is enabled.
    pub per_entry: Vec<Vec<usize>>,
}

fn sounds_like(cue_text: &str, range: (usize, usize), term: &str) -> bool {
    let srt = format!("1\n00:00:00,000 --> 00:00:01,000\n{cue_text}\n");
    let Ok(transcript) = parse_srt(&srt) else {
        return false;
    };
    let terms = [SessionTermEntry::new(term, Vec::new(), Vec::new())];
    let config = ExperimentalRetrievalConfig {
        han_sliding_window: true,
        ..ExperimentalRetrievalConfig::default()
    };
    retrieve_experimental_candidates(&transcript, &terms, &config)
        .iter()
        .any(|r| r.source_anchor.start_byte < range.1 && range.0 < r.source_anchor.end_byte)
}

pub fn analyse(lec: &Lecture, memory: &[MemEntry]) -> Analysis {
    let n = lec.issues.len();
    let mut per_issue: Vec<Option<Hit>> = vec![None; n];
    let mut per_entry: Vec<Vec<usize>> = vec![Vec::new(); memory.len()];
    for (e_idx, e) in memory.iter().enumerate() {
        for (i, is) in lec.issues.iter().enumerate() {
            if is.suggestion != e.suggestion {
                continue;
            }
            let exact = is.heard == e.heard;
            if !exact && !sounds_like(lec.cues[is.cue].text, is.range, &e.suggestion) {
                continue;
            }
            per_entry[e_idx].push(i);
            if e.enabled {
                let better = match per_issue[i] {
                    None => true,
                    Some(h) => exact && !h.exact,
                };
                if better {
                    per_issue[i] = Some(Hit {
                        entry: e_idx,
                        exact,
                    });
                }
            }
        }
    }
    Analysis {
        per_issue,
        per_entry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::pages::starter_memory;
    use crate::data::demo;

    #[test]
    fn starter_memory_matches_exact_and_by_sound() {
        let lec = demo();
        let a = analyse(&lec, &starter_memory());
        // 0: 踢度下架 never seen before, but sounds like the remembered 提讀下降.
        assert_eq!(
            a.per_issue[0],
            Some(Hit {
                entry: 0,
                exact: false
            })
        );
        // 4: BackPrepitation is the remembered spelling itself.
        assert_eq!(
            a.per_issue[4],
            Some(Hit {
                entry: 1,
                exact: true
            })
        );
        // 1/5/7: 收斂 is not remembered yet.
        assert!([1, 5, 7].iter().all(|&i| a.per_issue[i].is_none()));
        // Cross-script guess is not claimed as a sound match.
        assert!(a.per_issue[2].is_none());
    }

    #[test]
    fn learning_one_spelling_covers_other_spellings_and_disabling_removes_it() {
        let lec = demo();
        let mut mem = starter_memory();
        mem.push(MemEntry {
            heard: "收獵".into(),
            suggestion: "收斂".into(),
            scope: crate::app::pages::Scope::Course,
            source: "t".into(),
            uses: 0,
            enabled: true,
            fresh: false,
            source_issue: Some(1),
        });
        let a = analyse(&lec, &mem);
        assert_eq!(
            a.per_issue[1],
            Some(Hit {
                entry: 3,
                exact: true
            })
        );
        assert_eq!(
            a.per_issue[5],
            Some(Hit {
                entry: 3,
                exact: true
            })
        );
        assert_eq!(
            a.per_issue[7],
            Some(Hit {
                entry: 3,
                exact: false
            })
        ); // 收練
        mem[3].enabled = false;
        let b = analyse(&lec, &mem);
        assert!([1, 5, 7].iter().all(|&i| b.per_issue[i].is_none()));
        assert_eq!(b.per_entry[3], vec![1, 5, 7]);
    }
}
