use dioxus::prelude::*;
use iritsi_core::{unix_now, LocalTime};

use crate::{core, DATA_VERSION};

#[component]
pub fn WhenPicker(label: &'static str, mut at: Signal<Option<i64>>) -> Element {
    let range = use_memo(|| {
        DATA_VERSION.read();
        core().search_range().ok().flatten()
    });
    let shown = LocalTime::madrid(at().unwrap_or_else(unix_now));
    let mut set = move |date: String, time: String| {
        let Some(local) = LocalTime::parse(&date, &time) else {
            return;
        };
        if range().is_some_and(|(from, until)| local.date >= from && local.date <= until) {
            at.set(Some(local.to_unix()));
        }
    };

    rsx! {
        div { class: "when",
            span { class: "label", "{label}" }
            match range() {
                None => rsx! { p { class: "empty", "No timetable for upcoming days" } },
                Some((from, until)) => rsx! {
                    div { class: "when-row",
                        button {
                            class: if at().is_none() { "now active" } else { "now" },
                            onclick: move |_| at.set(None),
                            "Now"
                        }
                        input {
                            r#type: "date",
                            min: "{from}",
                            max: "{until}",
                            value: "{shown.date}",
                            onchange: move |event| set(event.value(), shown.time()),
                        }
                        input {
                            r#type: "time",
                            value: "{shown.time()}",
                            onchange: move |event| set(shown.date.to_string(), event.value()),
                        }
                    }
                },
            }
        }
    }
}
