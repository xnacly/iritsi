use std::time::Duration;

use dioxus::prelude::*;
use iritsi_core::{StopId, TimeDisplay};

use crate::{core, set_watch, DATA_VERSION, SETTINGS, WATCH};

#[component]
pub fn DepartureList(station: StopId) -> Element {
    let mut tick = use_signal(|| 0u64);
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            *tick.write() += 1;
        }
    });

    let departures = use_memo(move || {
        tick.read();
        DATA_VERSION.read();
        let limit = SETTINGS.read().departures;
        core()
            .departures(station, limit)
            .map_err(|err| err.to_string())
    });

    let display = SETTINGS.read().time_display;
    let watched = WATCH.read().as_ref().and_then(|watch| match watch {
        iritsi_core::Watch::Departure(leg) => Some(leg.trip_id),
        iritsi_core::Watch::Journey(_) => None,
    });
    let clock = display != TimeDisplay::Countdown;
    let countdown = display != TimeDisplay::Clock;

    rsx! {
        match &*departures.read() {
            Err(err) => rsx! { p { class: "error", "{err}" } },
            Ok(departures) if departures.is_empty() => rsx! {
                p { class: "empty", "No upcoming departures" }
            },
            Ok(departures) => rsx! {
                ul { class: "departures",
                    for departure in departures {
                        {
                            let (hour, minute) = departure.local_time();
                            let minutes = departure.minutes_until();
                            rsx! {
                                li {
                                    key: "{departure.trip_id}",
                                    class: if watched == Some(departure.trip_id) { "watched" } else { "" },
                                    title: "Watch this departure",
                                    onclick: {
                                        let departure = departure.clone();
                                        move |_| {
                                            if let Ok(Some(watch)) = core().watch_departure(station, &departure) {
                                                set_watch(Some(watch));
                                            }
                                        }
                                    },
                                    if clock {
                                        span { class: "time", "{hour:02}:{minute:02}" }
                                    }
                                    if !clock {
                                        span { class: "time",
                                            if minutes <= 0 { "now" } else { "{minutes} min" }
                                        }
                                    }
                                    span { class: "headsign", "{departure.headsign}" }
                                    if clock && countdown {
                                        span { class: "until",
                                            if minutes <= 0 { "now" } else { "{minutes} min" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
        }
    }
}
