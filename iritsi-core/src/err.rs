use iritsi_gtfs::GtfsErr;

#[derive(Debug)]
pub enum CoreErr {
    Gtfs(GtfsErr),
    Db(rusqlite::Error),
    IO(std::io::Error),
    Http(ureq::Error),
    InvalidDate(u32),
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
