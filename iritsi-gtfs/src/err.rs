#[derive(Debug)]
pub enum GtfsErr {
    /// Wraps errors occuring while zip decoding and decompressing the gtfs container
    ZipErr(zip::result::ZipError),
    MissingFile(&'static str),
    IO(std::io::Error),
    GtfsFormatErr(&'static str),
}

impl From<zip::result::ZipError> for GtfsErr {
    fn from(value: zip::result::ZipError) -> Self {
        GtfsErr::ZipErr(value)
    }
}

impl From<std::io::Error> for GtfsErr {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<csv::Error> for GtfsErr {
    fn from(_: csv::Error) -> Self {
        Self::GtfsFormatErr("csv parsing failure")
    }
}

impl std::fmt::Display for GtfsErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtfsErr::ZipErr(err) => write!(f, "zip: {err}"),
            GtfsErr::MissingFile(file) => write!(f, "missing {file}"),
            GtfsErr::IO(err) => write!(f, "io: {err}"),
            GtfsErr::GtfsFormatErr(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for GtfsErr {}
