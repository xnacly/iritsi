use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        p { "hello world hund" }
    }
}
