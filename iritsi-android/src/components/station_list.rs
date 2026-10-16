use dioxus::prelude::*;
use iritsi_core::{Station, StopId};

use crate::{
    components::{Icon, LineBadge},
    core, DATA_VERSION,
};

#[component]
pub fn StationList(mut query: Signal<String>, onselect: EventHandler<Station>) -> Element {
    let stations = use_memo(move || {
        DATA_VERSION.read();
        core()
            .search_stations(&query.read())
            .map_err(|err| err.to_string())
    });
    let favorites = use_memo(move || {
        DATA_VERSION.read();
        core().favorites().map_err(|err| err.to_string())
    });

    rsx! {
        input {
            class: "search",
            r#type: "search",
            placeholder: "SEARCH STATIONS",
            value: "{query}",
            oninput: move |event| query.set(event.value()),
        }
        if query.read().is_empty() {
            if let Ok(favorites) = &*favorites.read() {
                if !favorites.is_empty() {
                    h2 { class: "section", "Favorites" }
                    Rows { stations: favorites.clone(), onselect }
                }
            }
            h2 { class: "section", "All stations" }
        }
        match &*stations.read() {
            Err(err) => rsx! { p { class: "error", "{err}" } },
            Ok(stations) if stations.is_empty() => rsx! { p { class: "empty", "No stations found" } },
            Ok(stations) => rsx! { Rows { stations: stations.clone(), onselect } },
        }
    }
}

#[component]
fn Rows(stations: Vec<Station>, onselect: EventHandler<Station>) -> Element {
    rsx! {
        ul { class: "stations",
            for station in stations {
                li { key: "{station.id}",
                    button {
                        class: "station",
                        onclick: {
                            let station = station.clone();
                            move |_| onselect.call(station.clone())
                        },
                        span { class: "lines",
                            for line in station.lines.iter().copied() {
                                LineBadge { line }
                            }
                        }
                        span { class: "name", "{station.name}" }
                        span { class: "zone", "{station.zone_id}" }
                    }
                    FavoriteToggle { id: station.id, favorite: station.favorite }
                }
            }
        }
    }
}

#[component]
pub fn FavoriteToggle(id: StopId, favorite: bool) -> Element {
    let (icon, label) = if favorite {
        ("favorite", "Remove favorite")
    } else {
        ("favorite-off", "Add favorite")
    };
    rsx! {
        button {
            class: if favorite { "favorite on" } else { "favorite" },
            title: "{label}",
            onclick: move |_| {
                if core().set_favorite(id, !favorite).is_ok() {
                    *DATA_VERSION.write() += 1;
                }
            },
            Icon { name: icon }
        }
    }
}
