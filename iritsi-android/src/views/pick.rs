use dioxus::prelude::*;

use crate::{components::StationList, Route, Slot, FROM, TO};

#[component]
pub fn Pick(slot: Slot) -> Element {
    let query = use_signal(String::new);
    let title = match slot {
        Slot::From => "From",
        Slot::To => "To",
    };
    rsx! {
        h1 { "{title}" }
        StationList {
            query,
            onselect: move |station| {
                match slot {
                    Slot::From => *FROM.write() = Some(station),
                    Slot::To => *TO.write() = Some(station),
                }
                navigator().push(Route::Plan {});
            },
        }
    }
}
