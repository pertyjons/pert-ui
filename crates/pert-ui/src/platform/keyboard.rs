//! Native keyboard translation and the text-admission rule of the native `App`.
use crate::{CommandKey, CommandKeyInput, KeyAction, KeyCode, KeyDispatch, Modifiers};
use winit::keyboard::{Key, KeyCode as Native, NamedKey, NativeKeyCode, PhysicalKey};

/// Translates physical identity without layout-dependent aliases.
///
/// Numpad keys retain distinct opaque native codes regardless of NumLock.
/// Their named control meaning is supplied separately through CommandKeyInput;
/// KeyboardState captures that meaning for the physical press/release lifecycle.
/// Keys without a portable physical name use platform-scoped native codes,
/// which must not be persisted.
pub(super) fn translate(key: PhysicalKey, _logical: &Key) -> KeyCode {
    match key {
        PhysicalKey::Code(code) => match code {
            Native::KeyA => KeyCode::KeyA,
            Native::KeyB => KeyCode::KeyB,
            Native::KeyC => KeyCode::KeyC,
            Native::KeyD => KeyCode::KeyD,
            Native::KeyE => KeyCode::KeyE,
            Native::KeyF => KeyCode::KeyF,
            Native::KeyG => KeyCode::KeyG,
            Native::KeyH => KeyCode::KeyH,
            Native::KeyI => KeyCode::KeyI,
            Native::KeyJ => KeyCode::KeyJ,
            Native::KeyK => KeyCode::KeyK,
            Native::KeyL => KeyCode::KeyL,
            Native::KeyM => KeyCode::KeyM,
            Native::KeyN => KeyCode::KeyN,
            Native::KeyO => KeyCode::KeyO,
            Native::KeyP => KeyCode::KeyP,
            Native::KeyQ => KeyCode::KeyQ,
            Native::KeyR => KeyCode::KeyR,
            Native::KeyS => KeyCode::KeyS,
            Native::KeyT => KeyCode::KeyT,
            Native::KeyU => KeyCode::KeyU,
            Native::KeyV => KeyCode::KeyV,
            Native::KeyW => KeyCode::KeyW,
            Native::KeyX => KeyCode::KeyX,
            Native::KeyY => KeyCode::KeyY,
            Native::KeyZ => KeyCode::KeyZ,
            Native::Digit0 => KeyCode::Digit0,
            Native::Digit1 => KeyCode::Digit1,
            Native::Digit2 => KeyCode::Digit2,
            Native::Digit3 => KeyCode::Digit3,
            Native::Digit4 => KeyCode::Digit4,
            Native::Digit5 => KeyCode::Digit5,
            Native::Digit6 => KeyCode::Digit6,
            Native::Digit7 => KeyCode::Digit7,
            Native::Digit8 => KeyCode::Digit8,
            Native::Digit9 => KeyCode::Digit9,
            Native::F1 => KeyCode::F1,
            Native::F2 => KeyCode::F2,
            Native::F3 => KeyCode::F3,
            Native::F4 => KeyCode::F4,
            Native::F5 => KeyCode::F5,
            Native::F6 => KeyCode::F6,
            Native::F7 => KeyCode::F7,
            Native::F8 => KeyCode::F8,
            Native::F9 => KeyCode::F9,
            Native::F10 => KeyCode::F10,
            Native::F11 => KeyCode::F11,
            Native::F12 => KeyCode::F12,
            Native::Escape => KeyCode::Escape,
            Native::Enter => KeyCode::Enter,
            Native::Tab => KeyCode::Tab,
            Native::Space => KeyCode::Space,
            Native::Backspace => KeyCode::Backspace,
            Native::Delete => KeyCode::Delete,
            Native::Insert => KeyCode::Insert,
            Native::Home => KeyCode::Home,
            Native::End => KeyCode::End,
            Native::PageUp => KeyCode::PageUp,
            Native::PageDown => KeyCode::PageDown,
            Native::ArrowLeft => KeyCode::ArrowLeft,
            Native::ArrowRight => KeyCode::ArrowRight,
            Native::ArrowUp => KeyCode::ArrowUp,
            Native::ArrowDown => KeyCode::ArrowDown,
            Native::ShiftLeft => KeyCode::ShiftLeft,
            Native::ShiftRight => KeyCode::ShiftRight,
            Native::ControlLeft => KeyCode::ControlLeft,
            Native::ControlRight => KeyCode::ControlRight,
            Native::AltLeft => KeyCode::AltLeft,
            Native::AltRight => KeyCode::AltRight,
            Native::SuperLeft => KeyCode::SuperLeft,
            Native::SuperRight => KeyCode::SuperRight,
            Native::CapsLock => KeyCode::CapsLock,
            Native::Minus => KeyCode::Minus,
            Native::Equal => KeyCode::Equal,
            Native::BracketLeft => KeyCode::BracketLeft,
            Native::BracketRight => KeyCode::BracketRight,
            Native::Backslash => KeyCode::Backslash,
            Native::IntlBackslash => KeyCode::IntlBackslash,
            Native::ContextMenu => KeyCode::ContextMenu,
            Native::Semicolon => KeyCode::Semicolon,
            Native::Quote => KeyCode::Quote,
            Native::Backquote => KeyCode::Backquote,
            Native::Comma => KeyCode::Comma,
            Native::Period => KeyCode::Period,
            Native::Slash => KeyCode::Slash,
            other => KeyCode::Native(0x8000_0000 | other as u32),
        },
        PhysicalKey::Unidentified(code) => KeyCode::Native(match code {
            NativeKeyCode::Android(code) | NativeKeyCode::Xkb(code) => code,
            NativeKeyCode::MacOS(code) | NativeKeyCode::Windows(code) => u32::from(code),
            NativeKeyCode::Unidentified => 0,
        }),
    }
}

/// The layout identity of a logical key for command chords, or `None` for a
/// key with no single character or portable name.
pub(super) fn logical(key: &Key) -> Option<CommandKey> {
    match key {
        Key::Character(text) => {
            let mut chars = text.chars();
            let value = chars.next()?;
            if chars.next().is_none() {
                Some(CommandKey::Character(value))
            } else {
                None
            }
        }
        Key::Named(key) => Some(CommandKey::Named(match key {
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::Space => KeyCode::Space,
            NamedKey::Escape => KeyCode::Escape,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::ContextMenu => KeyCode::ContextMenu,
            NamedKey::ArrowLeft => KeyCode::ArrowLeft,
            NamedKey::ArrowRight => KeyCode::ArrowRight,
            NamedKey::ArrowUp => KeyCode::ArrowUp,
            NamedKey::ArrowDown => KeyCode::ArrowDown,
            NamedKey::Home => KeyCode::Home,
            NamedKey::End => KeyCode::End,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::F1 => KeyCode::F1,
            NamedKey::F2 => KeyCode::F2,
            NamedKey::F3 => KeyCode::F3,
            NamedKey::F4 => KeyCode::F4,
            NamedKey::F5 => KeyCode::F5,
            NamedKey::F6 => KeyCode::F6,
            NamedKey::F7 => KeyCode::F7,
            NamedKey::F8 => KeyCode::F8,
            NamedKey::F9 => KeyCode::F9,
            NamedKey::F10 => KeyCode::F10,
            NamedKey::F11 => KeyCode::F11,
            NamedKey::F12 => KeyCode::F12,
            _ => return None,
        })),
        _ => None,
    }
}

/// Whether key text is printable at all: it holds a character that is not a
/// control character. Enter's `"\r"` or Backspace's `"\u{8}"` are not text.
fn printable(text: Option<&str>) -> bool {
    text.is_some_and(|text| text.chars().any(|c| !c.is_control()))
}

/// The routing input for one native key event.
///
/// `composing` is set while the IME composes, for a dead key, and for text
/// typed with Control and Alt together (AltGr on Windows), so that text takes
/// precedence over command chords.
pub(super) fn command_input(
    physical: PhysicalKey,
    action: KeyAction,
    base: &Key,
    logical_key: &Key,
    text: Option<&str>,
    ime_composing: bool,
    modifiers: Modifiers,
) -> CommandKeyInput {
    let has_text = printable(text);
    CommandKeyInput {
        physical: translate(physical, logical_key),
        action,
        base: logical(base),
        logical: logical(logical_key),
        has_text,
        composing: ime_composing
            || matches!(logical_key, Key::Dead(_))
            || modifiers.control && modifiers.alt && has_text,
    }
}

/// The text a native key press inserts after routing, or `None`.
///
/// The runtime's [`KeyDispatch::deliver_text`] already covers routing and the
/// modifier rule. The host adds what only it knows: text is inserted on a
/// press or repeat, never while the IME composes (the IME commits that text
/// itself), and control characters are removed from it; text that is empty
/// afterwards inserts nothing.
pub(super) fn admitted_text(
    dispatch: &KeyDispatch,
    pressed: bool,
    ime_composing: bool,
    text: Option<&str>,
) -> Option<String> {
    if !dispatch.deliver_text || !pressed || ime_composing {
        return None;
    }
    let text: String = text?.chars().filter(|c| !c.is_control()).collect();
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyEvent, KeyTransition, Runtime, RuntimeError, TextField, Viewport};
    use std::time::Duration;

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }
    fn unnamed() -> Key {
        Key::Unidentified(winit::keyboard::NativeKey::Unidentified)
    }

    #[test]
    fn physical_table_names_letters_digits_modifiers_and_editing_keys() {
        for (native, expected) in [
            (Native::KeyA, KeyCode::KeyA),
            (Native::KeyZ, KeyCode::KeyZ),
            (Native::Digit0, KeyCode::Digit0),
            (Native::Digit9, KeyCode::Digit9),
            (Native::F10, KeyCode::F10),
            (Native::ShiftLeft, KeyCode::ShiftLeft),
            (Native::ControlRight, KeyCode::ControlRight),
            (Native::AltRight, KeyCode::AltRight),
            (Native::SuperLeft, KeyCode::SuperLeft),
            (Native::Enter, KeyCode::Enter),
            (Native::Insert, KeyCode::Insert),
            (Native::Backquote, KeyCode::Backquote),
            (Native::IntlBackslash, KeyCode::IntlBackslash),
            (Native::ContextMenu, KeyCode::ContextMenu),
        ] {
            assert_eq!(translate(PhysicalKey::Code(native), &unnamed()), expected);
        }
    }

    #[test]
    fn numpad_physical_codes_ignore_logical_navigation_and_remain_distinct() {
        let code = PhysicalKey::Code;
        assert_eq!(
            translate(code(Native::NumpadEnter), &Key::Named(NamedKey::Enter)),
            KeyCode::Native(0x8000_0000 | Native::NumpadEnter as u32)
        );
        // NumLock off: the layout reports navigation keys.
        for (native, named, expected) in [
            (Native::Numpad4, NamedKey::ArrowLeft, KeyCode::ArrowLeft),
            (Native::Numpad6, NamedKey::ArrowRight, KeyCode::ArrowRight),
            (Native::Numpad8, NamedKey::ArrowUp, KeyCode::ArrowUp),
            (Native::Numpad2, NamedKey::ArrowDown, KeyCode::ArrowDown),
            (Native::Numpad7, NamedKey::Home, KeyCode::Home),
            (Native::Numpad1, NamedKey::End, KeyCode::End),
            (Native::Numpad9, NamedKey::PageUp, KeyCode::PageUp),
            (Native::Numpad3, NamedKey::PageDown, KeyCode::PageDown),
            (Native::Numpad0, NamedKey::Insert, KeyCode::Insert),
            (Native::NumpadDecimal, NamedKey::Delete, KeyCode::Delete),
        ] {
            assert_eq!(
                translate(code(native), &Key::Named(named)),
                translate(code(native), &character("4")),
            );
            assert_ne!(translate(code(native), &Key::Named(named)), expected);
        }
        // NumLock on: the same physical native key delivers its digit as text.
        let four = translate(code(Native::Numpad4), &character("4"));
        assert!(matches!(four, KeyCode::Native(_)));
        assert_ne!(four, translate(code(Native::Numpad5), &character("5")));
        assert_ne!(four, KeyCode::Digit4);
    }

    #[test]
    fn unnamed_and_unidentified_codes_stay_distinct_native_codes() {
        let lang = translate(PhysicalKey::Code(Native::Lang1), &unnamed());
        let power = translate(PhysicalKey::Code(Native::Power), &unnamed());
        assert!(matches!(lang, KeyCode::Native(_)));
        assert!(matches!(power, KeyCode::Native(_)));
        assert_ne!(lang, power);
        assert_eq!(
            translate(
                PhysicalKey::Unidentified(NativeKeyCode::Xkb(0x42)),
                &unnamed()
            ),
            KeyCode::Native(0x42)
        );
        assert_eq!(
            translate(
                PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
                &unnamed()
            ),
            KeyCode::Native(0)
        );
    }

    #[test]
    fn logical_keys_name_characters_editing_keys_and_the_menu_key() {
        assert_eq!(logical(&character("q")), Some(CommandKey::Character('q')));
        assert_eq!(logical(&character("")), None);
        // A multi-character logical key (a ligature) has no single identity.
        assert_eq!(logical(&character("ab")), None);
        for (named, expected) in [
            (NamedKey::Insert, KeyCode::Insert),
            (NamedKey::Enter, KeyCode::Enter),
            (NamedKey::Delete, KeyCode::Delete),
            (NamedKey::F12, KeyCode::F12),
            (NamedKey::ContextMenu, KeyCode::ContextMenu),
        ] {
            assert_eq!(
                logical(&Key::Named(named)),
                Some(CommandKey::Named(expected))
            );
        }
        assert_eq!(logical(&Key::Named(NamedKey::Print)), None);
        assert_eq!(logical(&Key::Dead(Some('`'))), None);
    }

    #[test]
    fn command_input_marks_dead_keys_ime_and_control_alt_text_as_composing() {
        let physical = PhysicalKey::Code(Native::KeyQ);
        let q = character("q");
        let none = Modifiers::default();
        let plain = command_input(physical, KeyAction::Press, &q, &q, Some("q"), false, none);
        assert!(plain.has_text && !plain.composing);
        assert_eq!(plain.physical, KeyCode::KeyQ);
        assert_eq!(plain.base, Some(CommandKey::Character('q')));
        // Enter's carriage return is not printable text.
        let enter = Key::Named(NamedKey::Enter);
        let enter = command_input(
            PhysicalKey::Code(Native::Enter),
            KeyAction::Press,
            &enter,
            &enter,
            Some("\r"),
            false,
            none,
        );
        assert!(!enter.has_text);
        let dead = Key::Dead(Some('^'));
        assert!(command_input(physical, KeyAction::Press, &q, &dead, None, false, none).composing);
        let alt_gr = Modifiers {
            control: true,
            alt: true,
            ..none
        };
        let at = character("@");
        assert!(
            command_input(
                physical,
                KeyAction::Press,
                &q,
                &at,
                Some("@"),
                false,
                alt_gr
            )
            .composing
        );
        assert!(
            !command_input(physical, KeyAction::Press, &q, &q, None, false, alt_gr).composing,
            "Control+Alt without text is a chord"
        );
        assert!(command_input(physical, KeyAction::Press, &q, &q, Some("q"), true, none).composing);
    }

    fn dispatch(deliver_text: bool) -> KeyDispatch {
        KeyDispatch {
            event: KeyEvent {
                code: KeyCode::KeyQ,
                transition: KeyTransition::Pressed,
            },
            handled: deliver_text,
            deliver_text,
        }
    }

    #[test]
    fn admitted_text_needs_delivery_a_press_and_no_composition_and_drops_controls() {
        let admit = |deliver, pressed, composing, text| {
            admitted_text(&dispatch(deliver), pressed, composing, text)
        };
        assert_eq!(admit(true, true, false, Some("q")), Some("q".into()));
        assert_eq!(admit(false, true, false, Some("q")), None);
        assert_eq!(admit(true, false, false, Some("q")), None);
        assert_eq!(admit(true, true, true, Some("q")), None);
        assert_eq!(admit(true, true, false, None), None);
        assert_eq!(
            admit(true, true, false, Some("a\u{7f}b\u{85}")),
            Some("ab".into())
        );
        assert_eq!(admit(true, true, false, Some("\r")), None);
    }

    /// Types one key through the path the native `App` takes, from translated
    /// input through routing to insertion, with `held` modifier keys down.
    fn type_key(
        ui: &mut Runtime,
        held: &[KeyCode],
        logical_key: &Key,
        text: &str,
    ) -> Result<(), RuntimeError> {
        for key in held {
            ui.key_input(*key, KeyAction::Press)?;
        }
        let q = character("q");
        let input = command_input(
            PhysicalKey::Code(Native::KeyQ),
            KeyAction::Press,
            &q,
            logical_key,
            Some(text),
            false,
            ui.keyboard().modifiers(),
        );
        let routed = ui.command_key_input(input)?;
        if let Some(text) = admitted_text(&routed, true, false, Some(text)) {
            ui.text_input(&text)?;
        }
        ui.command_key_input(CommandKeyInput {
            action: KeyAction::Release,
            ..input
        })?;
        for key in held {
            ui.key_input(*key, KeyAction::Release)?;
        }
        Ok(())
    }

    #[test]
    fn native_key_path_inserts_plain_and_altgr_text_but_not_control_or_super_chords()
    -> Result<(), crate::AppError> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 100.0, 1.0)?)?;
        let value = ui.signal(String::new());
        let field = ui.mount_tree(crate::Element::new("field", TextField::new(value.clone())))?;
        ui.focus(&field)?;
        ui.flush(Duration::ZERO)?;
        type_key(&mut ui, &[], &character("q"), "q")?;
        type_key(&mut ui, &[KeyCode::ShiftLeft], &character("Q"), "Q")?;
        type_key(&mut ui, &[KeyCode::ControlLeft], &character("q"), "q")?;
        type_key(&mut ui, &[KeyCode::SuperLeft], &character("q"), "q")?;
        type_key(
            &mut ui,
            &[KeyCode::ControlLeft, KeyCode::AltRight],
            &character("@"),
            "@",
        )?;
        assert_eq!(value.get()?, "qQ@");
        Ok(())
    }
}
