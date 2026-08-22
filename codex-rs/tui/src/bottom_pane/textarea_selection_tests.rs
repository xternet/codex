use super::*;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::StatefulWidgetRef;

fn textarea(text: &str, cursor: usize) -> TextArea {
    let mut textarea = TextArea::new();
    textarea.set_selection_enabled(true);
    textarea.insert_str(text);
    textarea.set_cursor(cursor);
    textarea
}

fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn shifted_movement_extends_reverses_and_collapses_selection() {
    let mut textarea = textarea("alpha beta", 6);

    textarea.input(press(KeyCode::Right, KeyModifiers::SHIFT));
    assert_eq!(textarea.selected_text(), Some("b"));

    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    assert_eq!(textarea.selection_range(), None);

    textarea.input(press(
        KeyCode::Right,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    assert_eq!(textarea.selected_text(), Some("beta"));

    textarea.input(press(KeyCode::Left, KeyModifiers::NONE));
    assert_eq!(textarea.cursor(), 6);
    assert_eq!(textarea.selection_range(), None);
}

#[test]
fn shifted_vertical_and_line_movement_extend_selection() {
    let mut textarea = textarea("one two\nthree four", 13);

    textarea.input(press(KeyCode::Up, KeyModifiers::SHIFT));
    assert_eq!(textarea.selection_range(), Some(5..13));

    textarea.clear_selection();
    textarea.set_cursor(13);
    textarea.input(press(KeyCode::Home, KeyModifiers::SHIFT));
    assert_eq!(textarea.selected_text(), Some("three"));

    textarea.clear_selection();
    textarea.set_cursor(9);
    textarea.input(press(KeyCode::End, KeyModifiers::SHIFT));
    assert_eq!(textarea.selected_text(), Some("hree four"));
}

#[test]
fn selection_is_opt_in_and_empty_buffers_do_not_capture_shifted_movement() {
    let mut textarea = TextArea::new();
    assert!(!textarea.wants_selection_input(press(KeyCode::Left, KeyModifiers::SHIFT)));

    textarea.insert_str("modal");
    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    assert_eq!(textarea.selection_range(), None);

    textarea.set_selection_enabled(true);
    textarea.set_text_clearing_elements("");
    assert!(!textarea.wants_selection_input(press(KeyCode::Left, KeyModifiers::SHIFT)));
    textarea.set_text_clearing_elements("boundary");
    assert!(textarea.wants_selection_input(press(KeyCode::Left, KeyModifiers::SHIFT)));
    assert!(textarea.wants_selection_input(press(KeyCode::Up, KeyModifiers::SHIFT)));
}

#[test]
fn typing_and_deletion_replace_the_active_selection() {
    let mut textarea = textarea("abcd", 4);
    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));

    textarea.input(press(KeyCode::Char('Z'), KeyModifiers::SHIFT));
    assert_eq!(textarea.text(), "abZ");
    assert_eq!(textarea.cursor(), 3);
    assert_eq!(textarea.selection_range(), None);

    textarea.set_text_clearing_elements("abcd");
    textarea.set_cursor(4);
    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    textarea.input(press(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(textarea.text(), "abc");
    assert_eq!(textarea.selection_range(), None);
}

#[test]
fn line_kill_actions_kill_and_yank_the_active_selection() {
    let cases: [fn(&mut TextArea); 3] = [
        TextArea::kill_to_beginning_of_line,
        TextArea::kill_to_end_of_line,
        TextArea::kill_current_line,
    ];

    for (index, kill) in cases.into_iter().enumerate() {
        let mut textarea = textarea("abcDEFghi", if index == 0 { 6 } else { 3 });
        let direction = if index == 0 {
            KeyCode::Left
        } else {
            KeyCode::Right
        };
        for _ in 0..3 {
            textarea.input(press(direction, KeyModifiers::SHIFT));
        }

        kill(&mut textarea);
        assert_eq!(textarea.text(), "abcghi");
        assert_eq!(textarea.selection_range(), None);
        textarea.yank();
        assert_eq!(textarea.text(), "abcDEFghi");
    }
}

#[test]
fn selection_keeps_graphemes_and_elements_atomic() {
    let text = "Ae\u{301}👩\u{200d}💻Z";
    let mut unicode_textarea = textarea(text, text.len());

    unicode_textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    unicode_textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    assert_eq!(unicode_textarea.selected_text(), Some("👩\u{200d}💻Z"));

    let mut textarea = textarea("a[Image #1]b", "a[Image #1]".len());
    assert!(textarea.add_element_range(1..11).is_some());
    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    assert_eq!(textarea.selected_text(), Some("[Image #1]"));
    assert!(textarea.delete_selection());
    assert_eq!(textarea.text(), "ab");
    assert!(textarea.text_elements().is_empty());
}

#[test]
fn escape_and_whole_buffer_replacement_clear_selection() {
    let mut textarea = textarea("draft", 5);
    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    assert!(textarea.wants_selection_input(press(KeyCode::Esc, KeyModifiers::NONE)));

    textarea.input(press(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(textarea.selection_range(), None);

    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    textarea.set_text_clearing_elements("replacement");
    assert_eq!(textarea.selection_range(), None);
}

#[test]
fn vim_insert_escape_clears_selection_and_enters_normal_mode() {
    let mut textarea = textarea("draft", 5);
    textarea.set_vim_enabled(true);
    textarea.enter_vim_insert_mode();
    textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    assert_eq!(textarea.selected_text(), Some("t"));

    textarea.input(press(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(textarea.selection_range(), None);
    assert!(textarea.is_vim_normal_mode());
    assert!(!textarea.wants_selection_input(press(KeyCode::Left, KeyModifiers::SHIFT)));
}

#[test]
fn selection_rendering_spans_wrapped_text_and_newlines() {
    let mut textarea = textarea("ab界\ndef", "ab界\ndef".len());
    for _ in 0..5 {
        textarea.input(press(KeyCode::Left, KeyModifiers::SHIFT));
    }

    let area = Rect::new(0, 0, 4, 3);
    let mut buffer = Buffer::empty(area);
    let mut state = TextAreaState::default();
    StatefulWidgetRef::render_ref(&&textarea, area, &mut buffer, &mut state);

    let rendered = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| {
                    let cell = &buffer[(x, y)];
                    let selected = cell.style().add_modifier.contains(Modifier::REVERSED);
                    let symbol = if cell.symbol() == " " {
                        "·"
                    } else {
                        cell.symbol()
                    };
                    format!("{}{}", if selected { "*" } else { "." }, symbol)
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    insta::assert_snapshot!("textarea_selection_wraps_and_marks_newline", rendered);
}
