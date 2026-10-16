use dioxus::prelude::*;
use iritsi_core::Line;

#[component]
pub fn LineBadge(line: Line) -> Element {
    let name = line.name();
    rsx! {
        span { class: "line line-{name}", "{name}" }
    }
}
