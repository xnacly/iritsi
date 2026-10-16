use dioxus::prelude::*;
use iritsi_core::{unix_now, StopId, Watch};

use crate::{
    components::{DepartureList, WhenPicker},
    core, set_watch, PLAN_AT,
};

const TEST_CHANGE: &str = "San Ignazio";

#[component]
pub fn Journey(from: StopId, to: StopId) -> Element {
    let name = |id| {
        core()
            .station(id)
            .ok()
            .flatten()
            .map_or_else(|| id.to_string(), |station| station.name)
    };
    let (from_name, to_name) = (name(from), name(to));
    let mut missing = use_signal(|| false);

    let watch_test_trip = move |_| {
        let via = core()
            .search_stations(TEST_CHANGE)
            .ok()
            .and_then(|found| found.into_iter().next());
        let journey = via.and_then(|via| {
            core()
                .journey_via(from, via.id, to, PLAN_AT().unwrap_or_else(unix_now))
                .ok()
                .flatten()
        });
        missing.set(journey.is_none());
        if let Some(journey) = journey {
            set_watch(Some(Watch::Journey(journey)));
        }
    };

    rsx! {
        h1 { "{from_name} -> {to_name}" }
        p { class: "hint", "Route planning is not available yet" }
        WhenPicker { label: "Leave", at: PLAN_AT.signal() }
        button { class: "action", onclick: watch_test_trip, "Watch test trip via {TEST_CHANGE}" }
        if missing() {
            p { class: "empty", "No test trip via {TEST_CHANGE} found" }
        }
        h2 { class: "section", "Next trains from {from_name}" }
        DepartureList { station: from, at: PLAN_AT.signal() }
    }
}
