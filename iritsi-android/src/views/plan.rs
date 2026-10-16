use dioxus::prelude::*;
use iritsi_core::Station;

use crate::{
    components::{Icon, LineBadge, WhenPicker},
    Route, Slot, FROM, PLAN_AT, TO,
};

#[component]
pub fn Plan() -> Element {
    let from = FROM.read().clone();
    let to = TO.read().clone();
    let journey = match (&from, &to) {
        (Some(from), Some(to)) if from.id != to.id => Some(Route::Journey {
            from: from.id,
            to: to.id,
        }),
        _ => None,
    };

    rsx! {
        h1 { "Plan" }
        div { class: "plan",
            SlotButton { slot: Slot::From, station: from }
            button {
                class: "swap",
                title: "Swap",
                onclick: move |_| {
                    let from = FROM.read().clone();
                    *FROM.write() = TO.read().clone();
                    *TO.write() = from;
                },
                Icon { name: "swap" }
            }
            SlotButton { slot: Slot::To, station: to }
        }
        WhenPicker { label: "Leave", at: PLAN_AT.signal() }
        match journey {
            Some(route) => rsx! { Link { class: "primary", to: route, "Find route" } },
            None => rsx! { p { class: "hint", "Pick two different stations" } },
        }
    }
}

#[component]
fn SlotButton(slot: Slot, station: Option<Station>) -> Element {
    let label = match slot {
        Slot::From => "From",
        Slot::To => "To",
    };
    rsx! {
        Link { class: "slot", to: Route::Pick { slot },
            span { class: "label", "{label}" }
            match station {
                Some(station) => rsx! {
                    span { class: "name", "{station.name}" }
                    span { class: "lines",
                        for line in station.lines.iter().copied() {
                            LineBadge { line }
                        }
                    }
                },
                None => rsx! { span { class: "name unset", "Select station" } },
            }
        }
    }
}
