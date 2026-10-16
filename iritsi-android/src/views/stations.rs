use dioxus::prelude::*;
use iritsi_core::{Station, StopId};

use crate::{
    components::{DepartureList, FavoriteToggle, LineBadge, StationList},
    core, Route, DATA_VERSION, STATION_QUERY,
};

#[component]
pub fn Stations() -> Element {
    rsx! {
        h1 { "Stations" }
        StationList {
            query: STATION_QUERY.signal(),
            onselect: move |station: Station| {
                navigator().push(Route::StationDetail { id: station.id });
            },
        }
    }
}

#[component]
pub fn StationDetail(id: StopId) -> Element {
    let station = use_memo(move || {
        DATA_VERSION.read();
        core().station(id).ok().flatten()
    });
    rsx! {
        match station() {
            Some(station) => rsx! {
                header { class: "station-header",
                    div {
                        h1 { "{station.name}" }
                        p { class: "meta",
                            for line in station.lines.iter().copied() {
                                LineBadge { line }
                            }
                            span { class: "zone", "{station.zone_id}" }
                        }
                    }
                    FavoriteToggle { id, favorite: station.favorite }
                }
                h2 { class: "section", "Departures" }
                DepartureList { station: id }
            },
            None => rsx! { p { class: "error", "Unknown station {id}" } },
        }
    }
}
