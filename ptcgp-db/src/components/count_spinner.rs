use dioxus::prelude::*;

/// Owned-card count editor used in Card Catalog rows and Card Details.
///
/// `value` is the displayed value; when "Merge duplicate printings" is on it may be the
/// merged sum across a duplicate group. `stored_count` is the individual card version's
/// underlying count and is used solely to guard the decrement button — it equals `value`
/// when the merge setting is off. Pass `disabled = true` when multiple profiles are active:
/// the component becomes read-only and displays the aggregate sum.
#[component]
pub fn CountSpinner(
    /// Value to display. May be a merged sum when "Merge duplicate printings" is enabled.
    value: u32,
    /// Individual card version's stored count. Decrement is a no-op when this is 0.
    stored_count: u32,
    /// When true, renders as read-only and shows the aggregate value.
    disabled: bool,
    /// Called with the new individual card version count when the user edits.
    on_change: EventHandler<u32>,
) -> Element {
    // Local text buffer while the user is typing; None = not editing (show prop value).
    let mut edit: Signal<Option<String>> = use_signal(|| None);

    let displayed: String = {
        let guard = edit.read();
        match &*guard {
            Some(s) => s.clone(),
            None => value.to_string(),
        }
    };

    let decrement_disabled = disabled || stored_count == 0;

    let decrement_class = if decrement_disabled {
        "flex items-center justify-center w-7 h-7 rounded text-sm font-bold select-none \
         transition-colors bg-gray-100 dark:bg-gray-800 text-gray-400 dark:text-gray-600 \
         cursor-default"
    } else {
        "flex items-center justify-center w-7 h-7 rounded text-sm font-bold select-none \
         transition-colors bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 \
         dark:hover:bg-gray-600 text-gray-800 dark:text-gray-100 cursor-pointer"
    };
    let increment_class = if disabled {
        "flex items-center justify-center w-7 h-7 rounded text-sm font-bold select-none \
         transition-colors bg-gray-100 dark:bg-gray-800 text-gray-400 dark:text-gray-600 \
         cursor-default"
    } else {
        "flex items-center justify-center w-7 h-7 rounded text-sm font-bold select-none \
         transition-colors bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 \
         dark:hover:bg-gray-600 text-gray-800 dark:text-gray-100 cursor-pointer"
    };

    let input_class = if disabled {
        "w-12 text-center text-sm border rounded px-1 py-0.5 \
         bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 \
         border-gray-200 dark:border-gray-700 cursor-default"
    } else {
        "w-12 text-center text-sm border rounded px-1 py-0.5 \
         bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 \
         border-gray-300 dark:border-gray-600 \
         focus:outline-none focus:ring-1 focus:ring-blue-500"
    };

    rsx! {
        div { class: "inline-flex items-center gap-1",
            // Decrement
            button {
                r#type: "button",
                disabled: decrement_disabled,
                class: "{decrement_class}",
                onclick: move |_| {
                    if !decrement_disabled {
                        on_change.call(stored_count.saturating_sub(1));
                    }
                },
                "−"
            }

            // Numeric text input (not <input type="number"> — see DESIGN.md §Count Spinner)
            input {
                r#type: "text",
                value: "{displayed}",
                disabled,
                class: "{input_class}",
                oninput: move |evt| {
                    if !disabled {
                        edit.set(Some(evt.value()));
                    }
                },
                onfocus: move |_| {
                    if !disabled {
                        edit.set(Some(value.to_string()));
                    }
                },
                onblur: move |_| {
                    let raw = { (*edit.read()).clone() };
                    if let Some(s) = raw {
                        edit.set(None);
                        if let Ok(n) = s.trim().parse::<u64>() {
                            on_change.call(n.min(u64::from(u32::MAX)) as u32);
                        }
                        // Non-numeric: no on_change call → value reverts to prop on next render
                    }
                },
                onkeydown: move |evt| {
                    match evt.key() {
                        Key::Enter => {
                            let raw = { (*edit.read()).clone() };
                            if let Some(s) = raw {
                                edit.set(None);
                                if let Ok(n) = s.trim().parse::<u64>() {
                                    on_change.call(n.min(u64::from(u32::MAX)) as u32);
                                }
                            }
                        }
                        Key::Escape => edit.set(None),
                        _ => {}
                    }
                },
            }

            // Increment
            button {
                r#type: "button",
                disabled,
                class: "{increment_class}",
                onclick: move |_| {
                    if !disabled {
                        on_change.call(stored_count.saturating_add(1));
                    }
                },
                "+"
            }
        }
    }
}
