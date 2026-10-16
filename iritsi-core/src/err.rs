use iritsi_gtfs::GtfsErr;

#[derive(Debug)]
pub enum CoreErr {
    Gtfs(GtfsErr),
    Db(rusqlite::Error),
    IO(std::io::Error),
    Http(ureq::Error),
    NoDataDir,
    InvalidColor(String),
    InvalidDepartures(u32),
}

impl From<GtfsErr> for CoreErr {
    fn from(value: GtfsErr) -> Self {
        Self::Gtfs(value)
    }
}

impl From<rusqlite::Error> for CoreErr {
    fn from(value: rusqlite::Error) -> Self {
        Self::Db(value)
    }
}

impl From<std::io::Error> for CoreErr {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<ureq::Error> for CoreErr {
    fn from(value: ureq::Error) -> Self {
        Self::Http(value)
    }
}

impl std::fmt::Display for CoreErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreErr::Gtfs(err) => write!(f, "gtfs: {err}"),
            CoreErr::Db(err) => write!(f, "database: {err}"),
            CoreErr::IO(err) => write!(f, "io: {err}"),
            CoreErr::Http(err) => write!(f, "download: {err}"),
            CoreErr::NoDataDir => f.write_str("no data directory"),
            CoreErr::InvalidDepartures(count) => write!(f, "can not show {count} departures"),
            CoreErr::InvalidColor(color) => write!(f, "invalid color {color}, expected #rrggbb"),
        }
    }
}

impl std::error::Error for CoreErr {}
