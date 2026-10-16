use crate::{components::Icon, RefreshStatus, Route, Tab, REFRESH, TAB_ROUTES};
use dioxus::prelude::*;
use iritsi_core::Refresh;

const EDGE: f64 = 32.0;
const SWIPE: f64 = 80.0;

#[component]
pub fn Navbar() -> Element {
    let route = use_route::<Route>();
    use_effect(use_reactive!(|route| {
        let tab = route.tab() as usize;
        if TAB_ROUTES.peek()[tab] != route {
            TAB_ROUTES.write()[tab] = route;
        }
    }));
    let current = route.tab();
    let parent = route.parent();
    let mut swipe_from = use_signal(|| None::<f64>);

    rsx! {
        match &*REFRESH.read() {
            RefreshStatus::Checking => rsx! { p { class: "status", "Checking for updates" } },
            RefreshStatus::Done { refresh: Refresh::Updated, .. } => rsx! {
                Toast { class: "status", text: "Timetable updated" }
            },
            RefreshStatus::Done { refresh: Refresh::Unchanged, manual: true } => rsx! {
                Toast { class: "status", text: "Timetable up to date" }
            },
            RefreshStatus::Failed(_) => rsx! {
                Toast { class: "status offline", text: "Offline - stored timetable" }
            },
            _ => rsx! {},
        }
        main {
            ontouchstart: move |event| {
                let x = event.touches().first().map(|t| t.client_coordinates().x);
                swipe_from.set(x.filter(|&x| x < EDGE));
            },
            ontouchend: {
                let parent = parent.clone();
                move |event: TouchEvent| {
                    let Some(start) = swipe_from.take() else { return };
                    let end = event.touches_changed().first().map(|t| t.client_coordinates().x);
                    if let (Some(end), Some(parent)) = (end, parent.clone()) {
                        if end - start > SWIPE {
                            navigator().push(parent);
                        }
                    }
                }
            },
            if let Some(parent) = parent.clone() {
                Link { class: "back", to: parent,
                    Icon { name: "back" }
                    span { "Back" }
                }
            }
            Outlet::<Route> {}
        }
        nav { id: "tabs",
            TabLink { tab: Tab::Plan, current, icon: "tab-plan", label: "Plan" }
            TabLink { tab: Tab::Stations, current, icon: "tab-stations", label: "Stations" }
            TabLink { tab: Tab::About, current, icon: "tab-about", label: "About" }
        }
    }
}

#[component]
fn Toast(class: &'static str, text: &'static str) -> Element {
    rsx! {
        p { class: "{class} toast",
            span { "{text}" }
            button {
                class: "dismiss",
                title: "Dismiss",
                onclick: move |_| *REFRESH.write() = RefreshStatus::Idle,
                "[x]"
            }
        }
    }
}

#[component]
fn TabLink(tab: Tab, current: Tab, icon: &'static str, label: &'static str) -> Element {
    let active = tab == current;
    let to = if active {
        tab.root()
    } else {
        TAB_ROUTES.read()[tab as usize].clone()
    };
    rsx! {
        Link { to, class: if active { "active" } else { "" },
            Icon { name: icon }
            span { "{label}" }
        }
    }
}
