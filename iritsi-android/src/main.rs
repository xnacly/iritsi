use std::sync::OnceLock;

use dioxus::prelude::*;
use iritsi_core::{Iritsi, Progress, Refresh, Settings, Station, StopId};

use components::Splash;
use views::{About, Journey, Licenses, Navbar, Pick, Plan, StationDetail, Stations};

mod components;
mod licenses;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Plan {},

    #[route("/pick/:slot")]
    Pick { slot: Slot },

    #[route("/journey/:from/:to")]
    Journey { from: StopId, to: StopId },

    #[route("/stations")]
    Stations {},

    #[route("/stations/:id")]
    StationDetail { id: StopId },

    #[route("/about")]
    About {},

    #[route("/about/licenses")]
    Licenses {},
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Plan,
    Stations,
    About,
}

impl Route {
    pub fn parent(&self) -> Option<Route> {
        match self {
            Route::Pick { .. } | Route::Journey { .. } => Some(Route::Plan {}),
            Route::StationDetail { .. } => Some(Route::Stations {}),
            Route::Licenses {} => Some(Route::About {}),
            Route::Plan {} | Route::Stations {} | Route::About {} => None,
        }
    }

    pub fn tab(&self) -> Tab {
        match self {
            Route::Plan {} | Route::Pick { .. } | Route::Journey { .. } => Tab::Plan,
            Route::Stations {} | Route::StationDetail { .. } => Tab::Stations,
            Route::About {} | Route::Licenses {} => Tab::About,
        }
    }
}

impl Tab {
    pub(crate) fn root(self) -> Route {
        match self {
            Tab::Plan => Route::Plan {},
            Tab::Stations => Route::Stations {},
            Tab::About => Route::About {},
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Slot {
    From,
    To,
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Slot::From => "from",
            Slot::To => "to",
        })
    }
}

impl std::str::FromStr for Slot {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "from" => Ok(Slot::From),
            "to" => Ok(Slot::To),
            _ => Err(format!("unknown slot {s}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefreshStatus {
    Idle,
    Checking,
    Working(Progress),
    Done { refresh: Refresh, manual: bool },
    Failed(String),
}

pub static FROM: GlobalSignal<Option<Station>> = Signal::global(|| None);
pub static TO: GlobalSignal<Option<Station>> = Signal::global(|| None);
pub(crate) static TAB_ROUTES: GlobalSignal<[Route; 3]> =
    Signal::global(|| [Route::Plan {}, Route::Stations {}, Route::About {}]);
pub static STATION_QUERY: GlobalSignal<String> = Signal::global(String::new);
pub static SETTINGS: GlobalSignal<Settings> =
    Signal::global(|| core().settings().unwrap_or_default());
pub static DATA_VERSION: GlobalSignal<u64> = Signal::global(|| 0);
pub static REFRESH: GlobalSignal<RefreshStatus> = Signal::global(|| RefreshStatus::Idle);

static CORE: OnceLock<Result<Iritsi, String>> = OnceLock::new();

pub fn core() -> &'static Iritsi {
    CORE.get()
        .and_then(|core| core.as_ref().ok())
        .expect("core is opened before launch")
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const DEPARTURE_MONO: Asset = asset!("/assets/fonts/DepartureMono-Regular.woff2");
const MAPLE_MONO: Asset = asset!("/assets/fonts/MapleMono-Variable.ttf");

fn main() {
    CORE.get_or_init(|| Iritsi::open_default().map_err(|err| err.to_string()));

    #[cfg(feature = "desktop")]
    let launcher = dioxus::LaunchBuilder::new().with_cfg(
        dioxus::desktop::Config::new()
            .with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("iritsi")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(420.0, 820.0)),
            )
            .with_menu(None),
    );
    #[cfg(not(feature = "desktop"))]
    let launcher = dioxus::LaunchBuilder::new();

    launcher.launch(App);
}

const PAPER_TINT: f64 = 0.08;
const DARK_SHADE: f64 = 0.4;

fn mix(accent: &str, base: f64, amount: f64) -> String {
    let channel = |at: usize| {
        let channel = u8::from_str_radix(&accent[at..at + 2], 16).unwrap_or(0) as f64;
        (base + (channel - base) * amount).round() as u8
    };
    format!("#{:02x}{:02x}{:02x}", channel(1), channel(3), channel(5))
}

fn on_accent(accent: &str) -> &'static str {
    let channel = |at: usize| {
        let value = u8::from_str_radix(&accent[at..at + 2], 16).unwrap_or(0) as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
    if (luminance + 0.05) / 0.05 >= 1.05 / (luminance + 0.05) {
        "#000000"
    } else {
        "#ffffff"
    }
}

pub fn update_settings(change: impl FnOnce(&Iritsi) -> Result<(), iritsi_core::CoreErr>) {
    if change(core()).is_ok() {
        if let Ok(settings) = core().settings() {
            *SETTINGS.write() = settings;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefreshKind {
    Scheduled,
    Check,
    Redownload,
}

pub fn start_refresh(kind: RefreshKind) {
    let manual = kind != RefreshKind::Scheduled;
    if matches!(
        *REFRESH.peek(),
        RefreshStatus::Checking | RefreshStatus::Working(_)
    ) {
        return;
    }
    *REFRESH.write() = RefreshStatus::Checking;
    dioxus::core::spawn_forever(async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::task::spawn_blocking(move || {
            let progress = |progress| {
                let _ = tx.send(progress);
            };
            match kind {
                RefreshKind::Scheduled => core().refresh_with(progress),
                RefreshKind::Check => core().force_refresh_with(progress),
                RefreshKind::Redownload => core().redownload_timetable_with(progress),
            }
        });
        while let Some(progress) = rx.recv().await {
            *REFRESH.write() = RefreshStatus::Working(progress);
        }
        let status = match task.await {
            Ok(Ok(refresh)) => {
                if refresh == Refresh::Updated {
                    *DATA_VERSION.write() += 1;
                }
                RefreshStatus::Done { refresh, manual }
            }
            Ok(Err(err)) => RefreshStatus::Failed(err.to_string()),
            Err(err) => RefreshStatus::Failed(err.to_string()),
        };
        *REFRESH.write() = status;
    });
}

#[component]
fn SplashLayer() -> Element {
    match REFRESH() {
        RefreshStatus::Working(progress) => rsx! { Splash { progress } },
        _ => rsx! {},
    }
}

#[component]
fn App() -> Element {
    use_hook(|| start_refresh(RefreshKind::Scheduled));
    use_effect(|| {
        let settings = SETTINGS.read();
        let dark = mix(&settings.accent, 0.0, 1.0 - DARK_SHADE);
        let _ = document::eval(&format!(
            "const root = document.documentElement;
            root.style.setProperty('--orange', '{}');
            root.style.setProperty('--on-accent', '{}');
            root.style.setProperty('--paper', '{}');
            root.style.setProperty('--accent-dark', '{}');
            root.style.setProperty('--on-accent-dark', '{}');",
            settings.accent,
            on_accent(&settings.accent),
            mix(&settings.accent, 255.0, PAPER_TINT),
            dark,
            on_accent(&dark),
        ));
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Style {
            "@font-face {{ font-family: 'Departure Mono'; src: url('{DEPARTURE_MONO}') format('woff2'); font-display: block; }}
            @font-face {{ font-family: 'Maple Mono'; src: url('{MAPLE_MONO}') format('truetype'); font-weight: 100 800; font-display: block; }}"
        }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        match CORE.get() {
            Some(Err(err)) => rsx! {
                main { class: "error",
                    h1 { "iritsi could not start" }
                    p { "{err}" }
                }
            },
            _ => rsx! {
                Router::<Route> {}
                SplashLayer {}
            },
        }
    }
}
