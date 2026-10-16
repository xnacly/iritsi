use std::time::Duration;

use dioxus::prelude::*;
use iritsi_core::{unix_now, Line, Phase, Watch, WatchProgress, DEFAULT_LEAD};

use crate::{set_watch, WATCH};

fn countdown(secs: i64) -> String {
    let secs = secs.max(0);
    match secs / 3600 {
        0 => format!("{:02}:{:02}", secs / 60, secs % 60),
        hours => format!("{hours}:{:02}:{:02}", secs / 60 % 60, secs % 60),
    }
}

fn finished(watch: &Watch, now: i64) -> bool {
    now > watch.ends_at()
        || watch
            .current(now, DEFAULT_LEAD)
            .is_some_and(|alert| alert.phase == Phase::Done)
}

#[component]
pub fn WatchCard() -> Element {
    let mut now = use_signal(unix_now);
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            now.set(unix_now());
        }
    });

    use_effect(move || {
        let now = now();
        if WATCH
            .read()
            .as_ref()
            .is_some_and(|watch| finished(watch, now))
        {
            set_watch(None);
        }
    });

    let Some(watch) = WATCH.read().clone() else {
        return rsx! {};
    };
    let now = now();
    if finished(&watch, now) {
        return rsx! {};
    }
    let alert = watch.current(now, DEFAULT_LEAD);

    let (chip, title, text, target, progress) = match alert {
        Some(alert) => (
            alert.chip,
            alert.title,
            alert.text,
            alert.countdown_to,
            alert.progress,
        ),
        None => {
            let leg = &watch.legs()[0];
            let starts = watch.starts_at();
            (
                "Watching".to_owned(),
                match &watch {
                    Watch::Departure(_) => format!("Train to {}", leg.headsign),
                    Watch::Journey(journey) => {
                        format!("Trip to {}", journey.legs.last().unwrap().alight().name)
                    }
                },
                format!("From {}, alerts start 15 min before", leg.board().name),
                Some(starts),
                None,
            )
        }
    };

    rsx! {
        section { class: "watch",
            div { class: "watch-head",
                span { class: "watch-chip", "{chip}" }
                if let Some(target) = target {
                    span { class: "watch-countdown", {countdown(target - now)} }
                }
                button {
                    class: "dismiss",
                    title: "Stop watching",
                    onclick: move |_| set_watch(None),
                    "[x]"
                }
            }
            p { class: "watch-title", "{title}" }
            p { class: "watch-text", "{text}" }
            if let Some(progress) = progress {
                ProgressBar { progress }
            }
        }
    }
}

#[component]
fn ProgressBar(progress: WatchProgress) -> Element {
    let max = progress.max.max(1) as f64;
    let percent = |value: u32| value as f64 / max * 100.0;
    rsx! {
        div { class: "watch-progress",
            for segment in progress.segments.iter() {
                div {
                    class: match segment.line {
                        Some(Line::L1) => "segment l1",
                        Some(Line::L2) => "segment l2",
                        None => "segment wait",
                    },
                    style: "width: {percent(segment.length)}%",
                }
            }
            for point in progress.points.iter() {
                div { class: "point", style: "left: {percent(*point)}%" }
            }
            div { class: "tracker", style: "left: {percent(progress.value)}%" }
        }
    }
}
