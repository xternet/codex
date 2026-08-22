use std::ops::Range;

use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

use crate::key_hint::KeyBindingListExt;
use crate::key_hint::normalize_key_parts;
use crate::keymap::EditorKeymap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectionMotion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
}

#[derive(Debug, Default)]
pub(super) struct TextSelection {
    enabled: bool,
    anchor: Option<usize>,
}

impl TextSelection {
    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.clear();
        }
    }

    pub(super) fn clear(&mut self) {
        self.anchor = None;
    }

    pub(super) fn anchor(&self) -> Option<usize> {
        self.anchor
    }

    pub(super) fn finish_motion(&mut self, anchor: usize, cursor: usize) {
        self.anchor = (anchor != cursor).then_some(anchor);
    }

    pub(super) fn range(&self, cursor: usize) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        (anchor != cursor).then(|| anchor.min(cursor)..anchor.max(cursor))
    }

    pub(super) fn motion_for_event(
        &self,
        event: KeyEvent,
        keymap: &EditorKeymap,
    ) -> Option<SelectionMotion> {
        if !self.enabled || !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return None;
        }

        let (code, mut modifiers) = normalize_key_parts(event.code, event.modifiers);
        if !modifiers.contains(KeyModifiers::SHIFT) {
            return None;
        }
        modifiers.remove(KeyModifiers::SHIFT);
        let mut movement_event = event;
        movement_event.code = code;
        movement_event.modifiers = modifiers;

        if keymap.move_word_left.is_pressed(movement_event) {
            Some(SelectionMotion::WordLeft)
        } else if keymap.move_word_right.is_pressed(movement_event) {
            Some(SelectionMotion::WordRight)
        } else if keymap.move_left.is_pressed(movement_event) {
            Some(SelectionMotion::Left)
        } else if keymap.move_right.is_pressed(movement_event) {
            Some(SelectionMotion::Right)
        } else if keymap.move_up.is_pressed(movement_event) {
            Some(SelectionMotion::Up)
        } else if keymap.move_down.is_pressed(movement_event) {
            Some(SelectionMotion::Down)
        } else if keymap.move_line_start.is_pressed(movement_event) {
            Some(SelectionMotion::LineStart)
        } else if keymap.move_line_end.is_pressed(movement_event) {
            Some(SelectionMotion::LineEnd)
        } else {
            None
        }
    }
}
