use super::*;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::path::PathBuf;

fn select_left(chat: &mut ChatWidget) {
    chat.handle_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT));
}

#[tokio::test]
async fn shifted_movement_selects_composer_text_before_chat_shortcuts() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane.set_task_running(/*running*/ true);
    chat.input_queue
        .queued_user_messages
        .push_back(UserMessage::from("queued".to_string()).into());
    chat.bottom_pane
        .set_composer_text("draft".to_string(), Vec::new(), Vec::new());

    select_left(&mut chat);
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT));

    assert_eq!(chat.bottom_pane.composer_text(), "draft");
    assert_eq!(
        chat.bottom_pane.composer_selected_text().as_deref(),
        Some("draft")
    );
    assert_eq!(chat.input_queue.queued_user_messages.len(), 1);
    assert!(
        !std::iter::from_fn(|| rx.try_recv().ok())
            .any(|event| matches!(event, AppEvent::UpdateReasoningEffort(_)))
    );
}

#[tokio::test]
async fn escape_clears_selection_before_popup_or_running_turn_actions() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane.set_task_running(/*running*/ true);
    chat.handle_paste("/".to_string());
    select_left(&mut chat);
    assert_eq!(
        chat.bottom_pane.composer_selected_text().as_deref(),
        Some("/")
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(chat.bottom_pane.composer_text(), "/");
    assert_eq!(chat.bottom_pane.composer_selected_text(), None);
    assert!(
        !std::iter::from_fn(|| rx.try_recv().ok())
            .any(|event| matches!(event, AppEvent::CodexOp(Op::Interrupt)))
    );
}

#[tokio::test]
async fn paste_over_selection_reconciles_pending_pastes_and_images() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.handle_paste("x".repeat(1001));
    assert_eq!(chat.bottom_pane.composer_pending_pastes().len(), 1);
    select_left(&mut chat);
    chat.handle_paste("replacement".to_string());
    assert_eq!(chat.bottom_pane.composer_text(), "replacement");
    assert!(chat.bottom_pane.composer_pending_pastes().is_empty());

    let path = PathBuf::from("/tmp/selected-away.png");
    chat.bottom_pane.attach_image(path);
    select_left(&mut chat);
    chat.handle_paste("image gone".to_string());
    assert_eq!(chat.bottom_pane.composer_text(), "replacementimage gone");
    assert!(chat.bottom_pane.composer_local_images().is_empty());
}

#[tokio::test]
async fn text_and_remote_image_selection_are_mutually_exclusive() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let remote = "https://example.com/remote.png".to_string();
    chat.bottom_pane.set_remote_image_urls(vec![remote.clone()]);
    chat.bottom_pane
        .set_composer_text("draft".to_string(), Vec::new(), Vec::new());

    chat.handle_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    chat.handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    chat.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(chat.bottom_pane.remote_image_urls(), vec![remote]);
    assert_eq!(chat.bottom_pane.composer_text(), "raft");

    chat.bottom_pane
        .set_composer_text("draft".to_string(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::SHIFT));
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    chat.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert!(chat.bottom_pane.remote_image_urls().is_empty());
    assert_eq!(chat.bottom_pane.composer_text(), "draft");
}

#[tokio::test]
async fn plain_vertical_movement_does_not_recall_history_while_text_is_selected() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane
        .record_replayed_user_message_history(HistoryEntry::new("older".to_string()));
    chat.bottom_pane
        .set_composer_text("draft".to_string(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::SHIFT));

    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    assert_eq!(chat.bottom_pane.composer_text(), "draft");
    assert_eq!(chat.bottom_pane.composer_selected_text(), None);
}

#[tokio::test]
async fn disabling_input_clears_selection() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane
        .set_composer_text("read only".to_string(), Vec::new(), Vec::new());
    select_left(&mut chat);
    chat.bottom_pane
        .set_composer_input_enabled(/*enabled*/ false, /*placeholder*/ None);

    assert_eq!(chat.bottom_pane.composer_text(), "read only");
    assert_eq!(chat.bottom_pane.composer_selected_text(), None);
}
