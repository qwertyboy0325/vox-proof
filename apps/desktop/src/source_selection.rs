//! Display selection → source byte range. Not durable identity.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CueCharRange {
    pub cue_index: usize,
    pub start_char: usize,
    pub end_char: usize,
}

/// Transient cue glyph selection. Presentation state only; not durable identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CueSourceSelection {
    pub segment_position: usize,
    pub char_start: usize,
    pub char_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CuePointerDrag {
    pub segment_position: usize,
    pub origin_char: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CueSelectionSession {
    selection: Option<CueSourceSelection>,
    drag: Option<CuePointerDrag>,
    rendered_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSourceSpan {
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub observed_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    Empty,
    CrossCue,
    InvalidRange,
    ObservedMismatch,
}

pub fn char_range_to_utf8_bytes(
    text: &str,
    start_char: usize,
    end_char: usize,
) -> Option<(usize, usize)> {
    let mut starts: Vec<usize> = text.char_indices().map(|(index, _)| index).collect();
    starts.push(text.len());
    if start_char >= end_char || end_char >= starts.len() {
        return None;
    }
    Some((starts[start_char], starts[end_char]))
}

pub fn selected_span_text(text: &str, start_char: usize, end_char: usize) -> String {
    text.chars()
        .skip(start_char)
        .take(end_char.saturating_sub(start_char))
        .collect()
}

pub fn resolve_cue_selection(
    cue_text: &str,
    selection: CueCharRange,
    displayed_selected: &str,
) -> Result<ResolvedSourceSpan, SelectionError> {
    if selection.start_char >= selection.end_char {
        return Err(SelectionError::Empty);
    }
    let Some((start_byte, end_byte)) =
        char_range_to_utf8_bytes(cue_text, selection.start_char, selection.end_char)
    else {
        return Err(SelectionError::InvalidRange);
    };
    if !cue_text.is_char_boundary(start_byte) || !cue_text.is_char_boundary(end_byte) {
        return Err(SelectionError::InvalidRange);
    }
    let observed = &cue_text[start_byte..end_byte];
    if observed != displayed_selected {
        return Err(SelectionError::ObservedMismatch);
    }
    if observed.is_empty() {
        return Err(SelectionError::Empty);
    }
    Ok(ResolvedSourceSpan {
        segment_position: selection.cue_index,
        start_byte,
        end_byte,
        observed_text: observed.to_owned(),
    })
}

pub fn reject_cross_cue(anchor_cue: usize, other_cue: usize) -> Result<(), SelectionError> {
    if anchor_cue == other_cue {
        Ok(())
    } else {
        Err(SelectionError::CrossCue)
    }
}

pub fn selection_for_cue(
    live: Option<CueCharRange>,
    cached: Option<CueCharRange>,
    cue_index: usize,
) -> Option<CueCharRange> {
    live.filter(|selection| selection.cue_index == cue_index)
        .or_else(|| cached.filter(|selection| selection.cue_index == cue_index))
}

pub fn whole_cue_selection(cue_index: usize, cue_text: &str) -> CueCharRange {
    CueCharRange {
        cue_index,
        start_char: 0,
        end_char: cue_text.chars().count(),
    }
}

impl CueSourceSelection {
    pub fn from_drag(
        segment_position: usize,
        origin_char: usize,
        current_char: usize,
    ) -> Option<Self> {
        let (char_start, char_end) = if origin_char <= current_char {
            (origin_char, current_char)
        } else {
            (current_char, origin_char)
        };
        if char_start == char_end {
            None
        } else {
            Some(Self {
                segment_position,
                char_start,
                char_end,
            })
        }
    }

    pub fn is_nonempty(self) -> bool {
        self.char_start < self.char_end
    }

    pub fn fits_text(self, text: &str) -> bool {
        self.is_nonempty() && self.char_end <= text.chars().count()
    }

    pub fn as_char_range(self) -> CueCharRange {
        CueCharRange {
            cue_index: self.segment_position,
            start_char: self.char_start,
            end_char: self.char_end,
        }
    }
}

impl CueSelectionSession {
    pub fn selection(&self) -> Option<CueSourceSelection> {
        self.selection
    }

    pub fn selection_for_cue(&self, segment_position: usize) -> Option<CueSourceSelection> {
        self.selection.filter(|selection| {
            selection.segment_position == segment_position && selection.is_nonempty()
        })
    }

    pub fn is_dragging_cue(&self, segment_position: usize) -> bool {
        self.drag
            .is_some_and(|drag| drag.segment_position == segment_position)
    }

    pub fn begin_primary(&mut self, segment_position: usize, char_index: usize) {
        if self
            .selection
            .is_some_and(|selection| selection.segment_position != segment_position)
        {
            self.selection = None;
            self.rendered_text = None;
        }
        self.drag = Some(CuePointerDrag {
            segment_position,
            origin_char: char_index,
        });
    }

    pub fn extend_primary(&mut self, segment_position: usize, char_index: usize) {
        let Some(drag) = self.drag else {
            return;
        };
        if drag.segment_position != segment_position {
            return;
        }
        self.selection =
            CueSourceSelection::from_drag(segment_position, drag.origin_char, char_index);
        if self.selection.is_none() {
            self.rendered_text = None;
        }
    }

    pub fn finish_primary(&mut self, segment_position: usize, char_index: usize, dragged: bool) {
        if dragged {
            self.extend_primary(segment_position, char_index);
        } else if self.is_dragging_cue(segment_position) {
            if self
                .selection
                .is_some_and(|selection| selection.segment_position == segment_position)
            {
                self.selection = None;
                self.rendered_text = None;
            }
        }
        if self.is_dragging_cue(segment_position) {
            self.drag = None;
        }
    }

    pub fn sync_rendered_text(&mut self, segment_position: usize, text: &str) {
        let Some(selection) = self.selection else {
            return;
        };
        if selection.segment_position != segment_position {
            return;
        }
        if !selection.fits_text(text) {
            self.clear();
            return;
        }
        match &self.rendered_text {
            None => self.rendered_text = Some(text.to_owned()),
            Some(previous) if previous == text => {}
            Some(_) => self.clear(),
        }
    }

    pub fn clear(&mut self) {
        self.selection = None;
        self.drag = None;
        self.rendered_text = None;
    }
}

pub fn char_index_at_galley_pos(galley: &egui::Galley, local_pos: egui::Vec2, text: &str) -> usize {
    galley
        .cursor_from_pos(local_pos)
        .index
        .0
        .min(text.chars().count())
}

pub fn resolve_session_selection(
    session: &CueSelectionSession,
    segment_position: usize,
    cue_text: &str,
) -> Result<ResolvedSourceSpan, SelectionError> {
    let Some(selection) = session.selection_for_cue(segment_position) else {
        return Err(SelectionError::Empty);
    };
    let displayed = selected_span_text(cue_text, selection.char_start, selection.char_end);
    resolve_cue_selection(cue_text, selection.as_char_range(), &displayed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_selection_maps_to_source_bytes() {
        let text = "Hello Postgres world";
        let selection = CueCharRange {
            cue_index: 0,
            start_char: 6,
            end_char: 14,
        };
        let resolved = resolve_cue_selection(text, selection, "Postgres").unwrap();
        assert_eq!(resolved.segment_position, 0);
        assert_eq!(resolved.start_byte, 6);
        assert_eq!(resolved.end_byte, 14);
        assert_eq!(resolved.observed_text, "Postgres");
        assert_eq!(&text[resolved.start_byte..resolved.end_byte], "Postgres");
    }

    #[test]
    fn cjk_selection_maps_to_utf8_bytes() {
        let text = "歡迎使用轉錄校對工具";
        let start_char = 2;
        let end_char = 4;
        let displayed = selected_span_text(text, start_char, end_char);
        assert_eq!(displayed, "使用");
        let resolved = resolve_cue_selection(
            text,
            CueCharRange {
                cue_index: 3,
                start_char,
                end_char,
            },
            &displayed,
        )
        .unwrap();
        assert_eq!(resolved.segment_position, 3);
        assert_eq!(resolved.start_byte, "歡迎".len());
        assert_eq!(resolved.end_byte, "歡迎使用".len());
        assert_eq!(resolved.observed_text, "使用");
        assert_eq!(resolved.start_byte % 3, 0);
    }

    #[test]
    fn empty_selection_is_refused() {
        let text = "Postgres";
        assert_eq!(
            resolve_cue_selection(
                text,
                CueCharRange {
                    cue_index: 0,
                    start_char: 3,
                    end_char: 3,
                },
                "",
            ),
            Err(SelectionError::Empty)
        );
        assert!(char_range_to_utf8_bytes(text, 2, 2).is_none());
    }

    #[test]
    fn cross_cue_selection_is_unrepresentable() {
        assert_eq!(reject_cross_cue(0, 1), Err(SelectionError::CrossCue));
        assert_eq!(reject_cross_cue(2, 2), Ok(()));
    }

    #[test]
    fn displayed_text_mismatch_is_refused() {
        let text = "Hello Postgres";
        let selection = CueCharRange {
            cue_index: 0,
            start_char: 6,
            end_char: 14,
        };
        assert_eq!(
            resolve_cue_selection(text, selection, "PostgreSQL"),
            Err(SelectionError::ObservedMismatch)
        );
    }

    #[test]
    fn whole_cue_maps_full_source_range() {
        let text = "他";
        let selection = whole_cue_selection(1, text);
        let resolved = resolve_cue_selection(text, selection, "他").unwrap();
        assert_eq!(resolved.start_byte, 0);
        assert_eq!(resolved.end_byte, text.len());
        assert_eq!(resolved.observed_text, "他");
    }

    #[test]
    fn cached_selection_cannot_cross_cues() {
        let cue_a = CueCharRange {
            cue_index: 0,
            start_char: 0,
            end_char: 4,
        };
        let cue_b = CueCharRange {
            cue_index: 1,
            start_char: 2,
            end_char: 5,
        };
        assert_eq!(selection_for_cue(None, Some(cue_a), 1), None);
        assert_eq!(selection_for_cue(None, Some(cue_a), 0), Some(cue_a));
        assert_eq!(selection_for_cue(Some(cue_b), Some(cue_a), 1), Some(cue_b));
        assert_eq!(selection_for_cue(Some(cue_b), Some(cue_a), 0), Some(cue_a));
        assert_eq!(
            reject_cross_cue(cue_a.cue_index, cue_b.cue_index),
            Err(SelectionError::CrossCue)
        );
    }

    #[test]
    fn drag_ascii_left_to_right_and_right_to_left() {
        let mut session = CueSelectionSession::default();
        session.begin_primary(0, 6);
        session.extend_primary(0, 14);
        session.finish_primary(0, 14, true);
        let selection = session.selection_for_cue(0).unwrap();
        assert_eq!(
            selected_span_text(
                "Hello Postgres world",
                selection.char_start,
                selection.char_end
            ),
            "Postgres"
        );

        session.begin_primary(0, 14);
        session.extend_primary(0, 6);
        session.finish_primary(0, 6, true);
        let selection = session.selection_for_cue(0).unwrap();
        assert_eq!(
            selected_span_text(
                "Hello Postgres world",
                selection.char_start,
                selection.char_end
            ),
            "Postgres"
        );
    }

    #[test]
    fn drag_cjk_left_to_right_and_right_to_left() {
        let text = "上來一發";
        let mut session = CueSelectionSession::default();
        session.begin_primary(0, 0);
        session.extend_primary(0, 2);
        session.finish_primary(0, 2, true);
        let selection = session.selection_for_cue(0).unwrap();
        assert_eq!(
            selected_span_text(text, selection.char_start, selection.char_end),
            "上來"
        );

        session.begin_primary(0, 2);
        session.extend_primary(0, 0);
        session.finish_primary(0, 0, true);
        let selection = session.selection_for_cue(0).unwrap();
        assert_eq!(
            selected_span_text(text, selection.char_start, selection.char_end),
            "上來"
        );
    }

    #[test]
    fn click_only_is_not_a_correction_selection() {
        let mut session = CueSelectionSession::default();
        session.begin_primary(0, 1);
        session.finish_primary(0, 1, false);
        assert_eq!(session.selection_for_cue(0), None);
    }

    #[test]
    fn starting_selection_in_cue_b_replaces_cue_a() {
        let mut session = CueSelectionSession::default();
        session.begin_primary(0, 0);
        session.extend_primary(0, 2);
        session.finish_primary(0, 2, true);
        assert!(session.selection_for_cue(0).is_some());
        session.begin_primary(1, 0);
        assert_eq!(session.selection_for_cue(0), None);
        session.extend_primary(1, 2);
        session.finish_primary(1, 2, true);
        assert!(session.selection_for_cue(1).is_some());
        assert_eq!(session.selection_for_cue(0), None);
    }

    #[test]
    fn context_menu_cannot_apply_cue_a_to_cue_b() {
        let mut session = CueSelectionSession::default();
        session.begin_primary(0, 0);
        session.extend_primary(0, 2);
        session.finish_primary(0, 2, true);
        let before = session.selection();
        assert_eq!(session.selection_for_cue(1), None);
        assert_eq!(session.selection(), before);
    }

    #[test]
    fn repaint_and_focus_loss_do_not_mutate_selection() {
        let mut session = CueSelectionSession::default();
        session.begin_primary(3, 0);
        session.extend_primary(3, 2);
        session.finish_primary(3, 2, true);
        let before = session.clone();
        let _ = session.selection();
        let _ = session.selection_for_cue(3);
        assert_eq!(session, before);
    }

    #[test]
    fn source_text_replacement_invalidates_stale_selection() {
        let mut session = CueSelectionSession::default();
        session.begin_primary(0, 0);
        session.extend_primary(0, 4);
        session.finish_primary(0, 4, true);
        session.sync_rendered_text(0, "上來一發");
        session.sync_rendered_text(0, "上");
        assert_eq!(session.selection(), None);

        session.begin_primary(0, 0);
        session.extend_primary(0, 2);
        session.finish_primary(0, 2, true);
        session.sync_rendered_text(0, "上來一發");
        session.sync_rendered_text(0, "上來一發");
        assert!(session.selection_for_cue(0).is_some());
        session.sync_rendered_text(0, "發射一發");
        assert_eq!(session.selection(), None);
    }

    #[test]
    fn cjk_galley_positions_map_first_two_chars_to_上來() {
        let text = "上來一發";
        let ctx = egui::Context::default();
        let mut galley = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            galley = Some(ui.painter().layout_no_wrap(
                text.to_owned(),
                egui::FontId::proportional(16.0),
                ui.visuals().text_color(),
            ));
        });
        let galley = galley.expect("galley");
        let start = galley
            .pos_from_cursor(egui::text::CCursor::new(0))
            .min
            .to_vec2();
        let two = galley
            .pos_from_cursor(egui::text::CCursor::new(2))
            .min
            .to_vec2();
        assert_eq!(char_index_at_galley_pos(&galley, start, text), 0);
        assert_eq!(char_index_at_galley_pos(&galley, two, text), 2);
        let selection = CueSourceSelection::from_drag(
            0,
            char_index_at_galley_pos(&galley, start, text),
            char_index_at_galley_pos(&galley, two, text),
        )
        .unwrap();
        assert_eq!(
            selected_span_text(text, selection.char_start, selection.char_end),
            "上來"
        );
    }
}
