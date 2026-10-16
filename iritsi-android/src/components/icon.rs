use dioxus::prelude::*;

#[component]
pub fn Icon(name: &'static str) -> Element {
    let glyph = match name {
        "tab-plan" => "->",
        "tab-stations" => "[=]",
        "tab-about" => "?",
        "swap" => "^v",
        "back" => "<-",
        "favorite" => "[*]",
        "favorite-off" => "[ ]",
        _ => "",
    };
    rsx! {
        span { class: "icon icon-{name}", "aria-hidden": "true", "{glyph}" }
    }
}
