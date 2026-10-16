use std::time::Duration;

use crate::{Line, LocalTime, StopId};

pub const DEFAULT_LEAD: Duration = Duration::from_secs(15 * 60);
const BOARD_WINDOW: i64 = 2 * 60;
const DEPARTED_GRACE: i64 = 30;
const ARRIVED_GRACE: i64 = 2 * 60;

#[derive(Debug, Clone, PartialEq)]
pub struct StopCall {
    pub station: StopId,
    pub name: String,
    pub platform: StopId,
    pub arrives_at: i64,
    pub departs_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Leg {
    pub trip_id: u32,
    pub service_day_start: i64,
    pub line: Option<Line>,
    pub headsign: String,
    pub stops: Vec<StopCall>,
}

impl Leg {
    pub fn board(&self) -> &StopCall {
        &self.stops[0]
    }

    pub fn alight(&self) -> &StopCall {
        self.stops.last().expect("legs have at least one stop")
    }

    fn label(&self) -> String {
        match self.line {
            Some(line) => format!("{} to {}", line.name(), self.headsign),
            None => format!("Train to {}", self.headsign),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Journey {
    pub legs: Vec<Leg>,
}

impl Journey {
    pub fn departs_at(&self) -> i64 {
        self.legs[0].board().departs_at
    }

    pub fn arrives_at(&self) -> i64 {
        self.legs
            .last()
            .expect("journeys have legs")
            .alight()
            .arrives_at
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Watch {
    Departure(Leg),
    Journey(Journey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Upcoming,
    Board,
    Ride,
    GetOff,
    Transfer,
    Arrived,
    Done,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub length: u32,
    pub line: Option<Line>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub value: u32,
    pub max: u32,
    pub points: Vec<u32>,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    pub at: i64,
    pub phase: Phase,
    pub title: String,
    pub text: String,
    pub chip: String,
    pub countdown_to: Option<i64>,
    pub progress: Option<Progress>,
    pub buzz: bool,
}

impl Watch {
    pub fn legs(&self) -> &[Leg] {
        match self {
            Watch::Departure(leg) => std::slice::from_ref(leg),
            Watch::Journey(journey) => &journey.legs,
        }
    }

    pub fn ends_at(&self) -> i64 {
        match self {
            Watch::Departure(leg) => leg.board().departs_at + DEPARTED_GRACE,
            Watch::Journey(journey) => journey.arrives_at() + ARRIVED_GRACE,
        }
    }

    pub(crate) fn encode(&self) -> (&'static str, String) {
        let kind = match self {
            Watch::Departure(_) => "departure",
            Watch::Journey(_) => "journey",
        };
        let legs = self
            .legs()
            .iter()
            .map(|leg| {
                let alight = match self {
                    Watch::Departure(_) => String::new(),
                    Watch::Journey(_) => leg.alight().station.to_string(),
                };
                format!(
                    "{}:{}:{}:{}",
                    leg.trip_id,
                    leg.service_day_start,
                    leg.board().station,
                    alight
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        (kind, legs)
    }

    pub fn starts_at(&self) -> i64 {
        match self {
            Watch::Departure(leg) => leg.board().departs_at,
            Watch::Journey(journey) => journey.departs_at(),
        }
    }

    pub fn alerts(&self, lead: Duration) -> Vec<Alert> {
        match self {
            Watch::Departure(leg) => departure_alerts(leg, lead),
            Watch::Journey(journey) => journey_alerts(journey, lead),
        }
    }

    pub fn current(&self, now: i64, lead: Duration) -> Option<Alert> {
        self.alerts(lead)
            .into_iter()
            .take_while(|alert| alert.at <= now)
            .last()
    }
}

fn clock(unix: i64) -> String {
    let time = LocalTime::madrid(unix);
    format!("{:02}:{:02}", time.hour, time.minute)
}

fn departure_alerts(leg: &Leg, lead: Duration) -> Vec<Alert> {
    let board = leg.board();
    let departs = board.departs_at;
    let label = leg.label();
    vec![
        Alert {
            at: departs - lead.as_secs() as i64,
            phase: Phase::Upcoming,
            title: format!("{label} at {}", clock(departs)),
            text: format!("From {}", board.name),
            chip: clock(departs),
            countdown_to: Some(departs),
            progress: None,
            buzz: true,
        },
        Alert {
            at: departs - BOARD_WINDOW,
            phase: Phase::Board,
            title: format!("Board {label}"),
            text: format!("At {}, departs {}", board.name, clock(departs)),
            chip: "Board".to_owned(),
            countdown_to: Some(departs),
            progress: None,
            buzz: true,
        },
        done(departs + DEPARTED_GRACE),
    ]
}

fn done(at: i64) -> Alert {
    Alert {
        at,
        phase: Phase::Done,
        title: String::new(),
        text: String::new(),
        chip: String::new(),
        countdown_to: None,
        progress: None,
        buzz: false,
    }
}

fn journey_progress(journey: &Journey, now: i64) -> Progress {
    let start = journey.departs_at();
    let offset = |at: i64| (at - start).max(0) as u32;
    let max = offset(journey.arrives_at());
    let mut points = Vec::new();
    let mut segments = Vec::new();
    let mut covered = 0;
    for (idx, leg) in journey.legs.iter().enumerate() {
        if idx > 0 {
            let wait = offset(leg.board().departs_at) - covered;
            if wait > 0 {
                segments.push(Segment {
                    length: wait,
                    line: None,
                });
            }
        }
        for stop in &leg.stops {
            points.push(offset(stop.arrives_at.max(start)));
        }
        let end = offset(leg.alight().arrives_at);
        segments.push(Segment {
            length: end - offset(leg.board().departs_at),
            line: leg.line,
        });
        covered = end;
    }
    points.dedup();
    Progress {
        value: offset(now).min(max),
        max,
        points,
        segments,
    }
}

fn journey_alerts(journey: &Journey, lead: Duration) -> Vec<Alert> {
    let first = &journey.legs[0];
    let last = journey.legs.last().expect("journeys have legs");
    let destination = &last.alight().name;
    let start = journey.departs_at();
    let progress = |at| Some(journey_progress(journey, at));
    let mut alerts = vec![
        Alert {
            at: start - lead.as_secs() as i64,
            phase: Phase::Upcoming,
            title: format!("Trip to {destination} starts at {}", clock(start)),
            text: format!("{} from {}", first.label(), first.board().name),
            chip: clock(start),
            countdown_to: Some(start),
            progress: progress(start),
            buzz: true,
        },
        Alert {
            at: start - BOARD_WINDOW,
            phase: Phase::Board,
            title: format!("Board {}", first.label()),
            text: format!("At {}, departs {}", first.board().name, clock(start)),
            chip: "Board".to_owned(),
            countdown_to: Some(start),
            progress: progress(start),
            buzz: true,
        },
    ];

    for (idx, leg) in journey.legs.iter().enumerate() {
        let alight = leg.alight();
        let next_leg = journey.legs.get(idx + 1);
        for pair in leg.stops.windows(2) {
            let (from, to) = (&pair[0], &pair[1]);
            let getting_off = to.station == alight.station;
            let remaining = leg
                .stops
                .iter()
                .skip_while(|stop| stop.station != to.station)
                .count()
                - 1;
            alerts.push(if getting_off {
                Alert {
                    at: from.departs_at,
                    phase: Phase::GetOff,
                    title: format!("Get off at {}", to.name),
                    text: match next_leg {
                        Some(next) => format!(
                            "Then {} at {}",
                            next.label(),
                            clock(next.board().departs_at)
                        ),
                        None => format!("Arriving {}", clock(to.arrives_at)),
                    },
                    chip: to.name.clone(),
                    countdown_to: Some(to.arrives_at),
                    progress: progress(from.departs_at),
                    buzz: true,
                }
            } else {
                Alert {
                    at: from.departs_at,
                    phase: Phase::Ride,
                    title: format!("Next: {}", to.name),
                    text: format!(
                        "Get off at {} in {remaining} {}, {}",
                        alight.name,
                        if remaining == 1 { "stop" } else { "stops" },
                        clock(alight.arrives_at)
                    ),
                    chip: to.name.clone(),
                    countdown_to: Some(alight.arrives_at),
                    progress: progress(from.departs_at),
                    buzz: false,
                }
            });
        }
        if let Some(next) = next_leg {
            alerts.push(Alert {
                at: alight.arrives_at,
                phase: Phase::Transfer,
                title: format!("Change to {}", next.label()),
                text: format!(
                    "At {}, departs {}",
                    next.board().name,
                    clock(next.board().departs_at)
                ),
                chip: "Change".to_owned(),
                countdown_to: Some(next.board().departs_at),
                progress: progress(alight.arrives_at),
                buzz: true,
            });
            let board_at = next.board().departs_at - BOARD_WINDOW;
            if board_at > alight.arrives_at {
                alerts.push(Alert {
                    at: board_at,
                    phase: Phase::Board,
                    title: format!("Board {}", next.label()),
                    text: format!(
                        "At {}, departs {}",
                        next.board().name,
                        clock(next.board().departs_at)
                    ),
                    chip: "Board".to_owned(),
                    countdown_to: Some(next.board().departs_at),
                    progress: progress(board_at),
                    buzz: true,
                });
            }
        }
    }

    let arrives = journey.arrives_at();
    alerts.push(Alert {
        at: arrives,
        phase: Phase::Arrived,
        title: format!("Arrived at {destination}"),
        text: format!("Trip finished at {}", clock(arrives)),
        chip: "Arrived".to_owned(),
        countdown_to: None,
        progress: progress(arrives),
        buzz: true,
    });
    alerts.push(done(arrives + ARRIVED_GRACE));
    alerts.sort_by_key(|alert| alert.at);
    alerts
}
