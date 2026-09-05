use std::io::Cursor;

use zip::ZipArchive;

/// stop.txt specific csv row decoding
mod stops;

#[derive(Debug)]
pub enum GtfsErr {
    /// Wraps errors occuring while zip decoding and decompressing the gtfs container
    ZipErr(zip::result::ZipError),
    MissingFile(&'static str),
}

impl From<zip::result::ZipError> for GtfsErr {
    fn from(value: zip::result::ZipError) -> Self {
        GtfsErr::ZipErr(value)
    }
}

pub struct GtfsSchedule<'g> {
    archive: ZipArchive<Cursor<&'g [u8]>>,
    stops_idx: usize,
}

impl<'g> GtfsSchedule<'g> {
    /// Create a new GTFSShedule from a zip compressed gtfs conforming container
    pub fn new(src: &'g [u8]) -> Result<Self, GtfsErr> {
        let cursor = Cursor::new(src);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let mut stops = None;
        for idx in 0..archive.len() {
            let file = archive.by_index(idx)?;

            match file.name() {
                // TODO: add more files here
                "stops.txt" => stops = Some(idx),
                _ => continue,
            }
        }

        Ok(Self {
            archive,
            // TODO: add more files here
            stops_idx: stops.ok_or(GtfsErr::MissingFile("stops.txt"))?,
        })
    }
}

#[cfg(test)]
mod gtfs {}
