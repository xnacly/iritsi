use dioxus::prelude::*;
use iritsi_core::StopId;

use crate::{components::DepartureList, core};

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

    rsx! {
        h1 { "{from_name} -> {to_name}" }
        p { class: "hint", "Route planning is not available yet" }
        h2 { class: "section", "Next trains from {from_name}" }
        DepartureList { station: from }
    }
}
