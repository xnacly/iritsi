use std::io::Cursor;

use csv::ByteRecord;
use zip::ZipArchive;

use crate::{err::GtfsErr, stop_times::StopTimes, stops::Stops};

mod err;
mod num;

mod stop_times;
mod stops;

pub struct GtfsSchedule<'g> {
    archive: ZipArchive<Cursor<&'g [u8]>>,
    agency_idx: usize,
    calendar_idx: usize,
    calendar_dates_idx: usize,
    routes_idx: usize,
    trips_idx: usize,
    stop_times_idx: usize,
    stops_idx: usize,
    shapes_idx: usize,
    fare_attributes_idx: usize,
    fare_rules_idx: usize,
}

impl<'g> GtfsSchedule<'g> {
    /// Create a new GTFSShedule from a zip compressed gtfs conforming container
    pub fn new(src: &'g [u8]) -> Result<Self, err::GtfsErr> {
        let mut archive = zip::ZipArchive::new(Cursor::new(src))?;

        let mut agency = None;
        let mut calendar = None;
        let mut calendar_dates = None;
        let mut routes = None;
        let mut trips = None;
        let mut stop_times = None;
        let mut stops = None;
        let mut shapes = None;
        let mut fare_attributes = None;
        let mut fare_rules = None;

        for idx in 0..archive.len() {
            let file = archive.by_index(idx)?;

            match file.name() {
                "agency.txt" => agency = Some(idx),
                "calendar.txt" => calendar = Some(idx),
                "calendar_dates.txt" => calendar_dates = Some(idx),
                "routes.txt" => routes = Some(idx),
                "trips.txt" => trips = Some(idx),
                "stop_times.txt" => stop_times = Some(idx),
                "stops.txt" => stops = Some(idx),
                "shapes.txt" => shapes = Some(idx),
                "fare_attributes.txt" => fare_attributes = Some(idx),
                "fare_rules.txt" => fare_rules = Some(idx),
                _ => continue,
            }
        }

        Ok(Self {
            archive,
            agency_idx: agency.ok_or(err::GtfsErr::MissingFile("agency.txt"))?,
            calendar_idx: calendar.ok_or(err::GtfsErr::MissingFile("calendar.txt"))?,
            calendar_dates_idx: calendar_dates
                .ok_or(err::GtfsErr::MissingFile("calendar_dates.txt"))?,
            routes_idx: routes.ok_or(err::GtfsErr::MissingFile("routes.txt"))?,
            trips_idx: trips.ok_or(err::GtfsErr::MissingFile("trips.txt"))?,
            stop_times_idx: stop_times.ok_or(err::GtfsErr::MissingFile("stop_times.txt"))?,
            stops_idx: stops.ok_or(err::GtfsErr::MissingFile("stops.txt"))?,
            shapes_idx: shapes.ok_or(err::GtfsErr::MissingFile("shapes.txt"))?,
            fare_attributes_idx: fare_attributes
                .ok_or(err::GtfsErr::MissingFile("fare_attributes.txt"))?,
            fare_rules_idx: fare_rules.ok_or(err::GtfsErr::MissingFile("fare_rules.txt"))?,
        })
    }

    fn reader_from_file(&mut self, idx: usize) -> Result<csv::Reader<Cursor<Vec<u8>>>, GtfsErr> {
        let mut file = self.archive.by_index(idx)?;
        let mut data = Vec::with_capacity(file.size() as usize);
        let _ = std::io::Read::read_to_end(&mut file, &mut data)?;
        Ok(csv::ReaderBuilder::new().from_reader(Cursor::new(data)))
    }

    pub fn stops(&mut self) -> Result<Stops<Cursor<Vec<u8>>>, GtfsErr> {
        Ok(Stops {
            reader: self.reader_from_file(self.stops_idx)?,
            reused_record: ByteRecord::with_capacity(64, 8),
        })
    }

    pub fn stop_times(&mut self) -> Result<StopTimes, GtfsErr> {
        let mut file = self.archive.by_index(self.stop_times_idx)?;
        let mut data = Vec::with_capacity(file.size() as usize);
        let _ = std::io::Read::read_to_end(&mut file, &mut data)?;
        Ok(data.into())
    }
}

#[cfg(test)]
mod gtfs {}
