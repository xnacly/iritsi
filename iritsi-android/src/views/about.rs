use dioxus::prelude::*;

use std::time::Duration;

use iritsi_core::{
    FeedSource, TimeDisplay, TimetableInfo, UpdateInterval, DEFAULT_ACCENT, DEPARTURE_CHOICES,
};

use crate::{
    core, licenses::LICENSES, start_refresh, update_settings, RefreshKind, RefreshStatus, Route,
    DATA_VERSION, REFRESH, SETTINGS,
};

const APP_ICON: Asset = asset!("/assets/icons/rings.png");

#[component]
pub fn About() -> Element {
    rsx! {
        header { class: "about-header",
            img { class: "app-icon", src: APP_ICON, alt: "iritsi" }
            h1 { "iritsi" }
        }
        SettingsCard {}
        DataCard {}
        div { class: "card",
            h2 { "Data source" }
            // or https://ctb-siri.s3.eu-south-2.amazonaws.com/metro-bilbao-trip-updates.xml if the day
            // comes
            p { "All data is sourced from the OPEN DATA Initiative at: "
                a {
                    href: "https://www.metrobilbao.eus/en/open-data/dataset/api-guide",
                    "Metro Bilbao API"
                }
                "."
            }
        }
        div { class: "card",
            h2 { "Source Code" }
            p { "The source code is available at: "
                a { href:"https://github.com/xnacly/iritsi", "github.com/xnacly/iritsi" }
            }
        }
        div { class: "card",
            h2 { "Licenses" }
            p { "iritsi is built on open source software and fonts. "
                Link { to: Route::Licenses {}, "View licenses" }
            }
        }
        div { class: "card",
            p { "Made with love by " a { href:"https://xnacly.me", "xnacly" } }
        }
        TimetableCard {}
        AppCard {}
    }
}

fn interval_label(interval: UpdateInterval) -> String {
    match interval {
        UpdateInterval::Manual => "Only manually".to_owned(),
        UpdateInterval::Every(every) => match every.as_secs() / 3600 {
            1 => "Every hour".to_owned(),
            24 => "Every day".to_owned(),
            168 => "Every week".to_owned(),
            hours if hours % 24 == 0 => format!("Every {} days", hours / 24),
            hours => format!("Every {hours} hours"),
        },
    }
}

fn time_display_label(display: TimeDisplay) -> &'static str {
    match display {
        TimeDisplay::Both => "08:03 and in 4 min",
        TimeDisplay::Clock => "08:03",
        TimeDisplay::Countdown => "in 4 min",
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Danger {
    ClearFavorites,
    ResetSettings,
}

#[component]
fn DataCard() -> Element {
    let mut armed = use_signal(|| None::<Danger>);
    let busy = matches!(
        *REFRESH.read(),
        RefreshStatus::Checking | RefreshStatus::Working(_)
    );
    let mut confirm = move |danger: Danger, action: fn()| {
        if armed() == Some(danger) {
            armed.set(None);
            action();
        } else {
            armed.set(Some(danger));
        }
    };

    rsx! {
        div { class: "card",
            h2 { "Data" }
            div { class: "actions",
                button {
                    class: "action",
                    disabled: busy,
                    onclick: move |_| start_refresh(RefreshKind::Redownload),
                    "Redownload timetable"
                }
                button {
                    class: if armed() == Some(Danger::ClearFavorites) { "action danger" } else { "action" },
                    onclick: move |_| confirm(Danger::ClearFavorites, || {
                        if core().clear_favorites().is_ok() {
                            *DATA_VERSION.write() += 1;
                        }
                    }),
                    if armed() == Some(Danger::ClearFavorites) { "Tap again to clear favorites" } else { "Clear favorites" }
                }
                button {
                    class: if armed() == Some(Danger::ResetSettings) { "action danger" } else { "action" },
                    onclick: move |_| confirm(Danger::ResetSettings, || update_settings(|core| core.reset_settings())),
                    if armed() == Some(Danger::ResetSettings) { "Tap again to reset settings" } else { "Reset settings" }
                }
            }
        }
    }
}

#[component]
fn SettingsCard() -> Element {
    let settings = SETTINGS.read().clone();
    let selected = UpdateInterval::CHOICES
        .iter()
        .position(|&choice| choice == settings.update_interval);

    rsx! {
        div { class: "card",
            h2 { "Settings" }
            label { class: "setting",
                span { "Update timetable" }
                select {
                    class: "select",
                    onchange: move |event| {
                        if let Some(&interval) = event
                            .value()
                            .parse::<usize>()
                            .ok()
                            .and_then(|idx| UpdateInterval::CHOICES.get(idx))
                        {
                            update_settings(|core| core.set_update_interval(interval));
                        }
                    },
                    if selected.is_none() {
                        option { selected: true, disabled: true, {interval_label(settings.update_interval)} }
                    }
                    for (idx, choice) in UpdateInterval::CHOICES.into_iter().enumerate() {
                        option { value: "{idx}", selected: Some(idx) == selected, {interval_label(choice)} }
                    }
                }
            }
            label { class: "setting",
                span { "Departures shown" }
                select {
                    class: "select",
                    onchange: move |event| {
                        if let Ok(count) = event.value().parse::<u32>() {
                            update_settings(|core| core.set_departures(count));
                        }
                    },
                    for count in DEPARTURE_CHOICES {
                        option { value: "{count}", selected: count == settings.departures, "{count} trains" }
                    }
                }
            }
            label { class: "setting",
                span { "Time display" }
                select {
                    class: "select",
                    onchange: move |event| {
                        if let Some(display) = TimeDisplay::ALL.into_iter().find(|d| d.name() == event.value()) {
                            update_settings(|core| core.set_time_display(display));
                        }
                    },
                    for display in TimeDisplay::ALL {
                        option { value: display.name(), selected: display == settings.time_display, {time_display_label(display)} }
                    }
                }
            }
            div { class: "setting",
                span { "Accent color" }
                div { class: "accent",
                    input {
                        r#type: "color",
                        value: "{settings.accent}",
                        oninput: move |event| {
                            let accent = event.value();
                            update_settings(|core| core.set_accent(Some(&accent)));
                        },
                    }
                    button {
                        class: "action",
                        disabled: settings.accent == DEFAULT_ACCENT,
                        onclick: move |_| update_settings(|core| core.set_accent(None)),
                        "Reset"
                    }
                }
            }
        }
    }
}

#[component]
fn TimetableCard() -> Element {
    let info = use_memo(|| {
        DATA_VERSION.read();
        REFRESH.read();
        core().timetable_info().ok()
    });
    let busy = matches!(
        *REFRESH.read(),
        RefreshStatus::Checking | RefreshStatus::Working(_)
    );
    let or_never = |value: Option<String>| value.unwrap_or_else(|| "never".to_owned());

    rsx! {
        div { class: "card",
            h2 { "Timetable" }
            match info() {
                None => rsx! { p { "No timetable stored" } },
                Some(info) => rsx! {
                    dl { class: "facts",
                        dt { "Last update" }
                        dd { {or_never(info.imported_at.map(|t| t.to_string()))} }
                        dt { "Last check" }
                        dd { {or_never(info.checked_at.map(|t| t.to_string()))} }
                        dt { "Source" }
                        dd {
                            match info.source {
                                Some(FeedSource::Bundled) => "Bundled with app",
                                Some(FeedSource::Downloaded) => "Metro Bilbao",
                                None => "-",
                            }
                        }
                        dt { "Update took" }
                        dd { {update_time(&info)} }
                        dt { "Valid" }
                        dd {
                            match (info.valid_from, info.valid_until) {
                                (Some(from), Some(until)) => rsx! { "{from} - {until}" },
                                _ => rsx! { "-" },
                            }
                        }
                        dt { "Feed size" }
                        dd { {info.feed_size.map_or_else(|| "-".to_owned(), size)} }
                        dt { "Database" }
                        dd { {size(info.database_size)} }
                        dt { "Preferences" }
                        dd { {size(info.user_database_size)} }
                        dt { "Stations" }
                        dd { "{info.stations}" }
                        dt { "Trips" }
                        dd { "{info.trips}" }
                        dt { "Stop times" }
                        dd { "{info.stop_times}" }
                    }
                },
            }
            button {
                class: "action",
                disabled: busy,
                onclick: move |_| start_refresh(RefreshKind::Check),
                if busy { "Checking..." } else { "Check for updates now" }
            }
        }
    }
}

fn seconds(duration: Duration) -> String {
    format!("{:.2} s", duration.as_secs_f64())
}

fn update_time(info: &TimetableInfo) -> String {
    match (info.update_time(), info.download_time, info.import_time) {
        (Some(total), Some(download), Some(import)) => format!(
            "{} (download {}, import {})",
            seconds(total),
            seconds(download),
            seconds(import)
        ),
        (Some(total), None, _) => format!("{} (import only)", seconds(total)),
        _ => "-".to_owned(),
    }
}

fn size(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

#[component]
fn AppCard() -> Element {
    let favorites = use_memo(|| {
        DATA_VERSION.read();
        core().favorites().map(|f| f.len()).unwrap_or(0)
    });
    let platform = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    rsx! {
        div { class: "card",
            h2 { "App" }
            dl { class: "facts",
                dt { "App version" }
                dd { {env!("CARGO_PKG_VERSION")} }
                dt { "Core version" }
                dd { {iritsi_core::VERSION} }
                dt { "Platform" }
                dd { "{platform}" }
                dt { "Favorites" }
                dd { "{favorites}" }
            }
        }
    }
}

#[component]
pub fn Licenses() -> Element {
    let mut open = use_signal(|| None::<usize>);
    rsx! {
        h1 { "Licenses" }
        ul { class: "licenses",
            for (idx, license) in LICENSES.iter().enumerate() {
                li { key: "{license.name}",
                    button {
                        class: "license",
                        onclick: move |_| open.set(if open() == Some(idx) { None } else { Some(idx) }),
                        span { class: "name", "{license.name}" }
                        span { class: "kind", "{license.kind}" }
                    }
                    if open() == Some(idx) {
                        div { class: "license-text",
                            a { href: license.url, "{license.url}" }
                            pre { "{license.text}" }
                        }
                    }
                }
            }
        }
    }
}
