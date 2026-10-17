use dioxus::prelude::*;
use iritsi_core::Progress;

const APP_ICON: Asset = asset!("/assets/icons/rings.png");

#[component]
pub fn Splash(progress: Progress) -> Element {
    let (phase, percent, detail) = match progress {
        Progress::Downloading { received, total } => {
            let kb = received / 1024;
            match total {
                Some(total) if total > 0 => (
                    "Downloading timetable",
                    Some((received * 100 / total).min(100)),
                    format!("{kb} / {} KB", total / 1024),
                ),
                _ => ("Downloading timetable", None, format!("{kb} KB")),
            }
        }
        Progress::Importing => ("Importing timetable", None, String::new()),
    };

    rsx! {
        div { class: "splash",
            img { class: "splash-icon", src: APP_ICON, alt: "iritsi" }
            h1 { "iritsi" }
            p { class: "splash-phase", "{phase}" }
            div { class: if percent.is_some() { "progress" } else { "progress busy" },
                div {
                    class: "fill",
                    style: "width: {percent.unwrap_or(100)}%",
                }
            }
            p { class: "splash-detail",
                match percent {
                    Some(percent) => rsx! { "{percent}% - {detail}" },
                    None => rsx! { "{detail}" },
                }
            }
        }
    }
}
