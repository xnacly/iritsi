use std::io::Cursor;

use csv::ByteRecord;
use zip::ZipArchive;

use crate::{
    calendar::Calendars, calendar_dates::CalendarDates, routes::Routes, shapes::Shapes,
    stop_times::StopTimes, stops::Stops, trips::Trips,
};

mod err;
mod num;

mod calendar;
mod calendar_dates;
mod routes;
mod shapes;
mod stop_times;
mod stops;
mod trips;

pub use calendar::{Calendar, Weekday};
pub use calendar_dates::{CalendarDate, ServiceException};
pub use err::GtfsErr;
pub use routes::Route;
pub use shapes::ShapePoint;
pub use stop_times::StopTime;
pub use stops::{LocationType, Stop, StopId};
pub use trips::Trip;

pub struct GtfsSchedule<'g> {
    archive: ZipArchive<Cursor<&'g [u8]>>,
    calendar_idx: usize,
    calendar_dates_idx: usize,
    routes_idx: usize,
    trips_idx: usize,
    stop_times_idx: usize,
    stops_idx: usize,
    shapes_idx: usize,
}

impl<'g> GtfsSchedule<'g> {
    /// Create a new GTFSShedule from a zip compressed gtfs conforming container
    pub fn new(src: &'g [u8]) -> Result<Self, err::GtfsErr> {
        let mut archive = zip::ZipArchive::new(Cursor::new(src))?;

        let mut calendar = None;
        let mut calendar_dates = None;
        let mut routes = None;
        let mut trips = None;
        let mut stop_times = None;
        let mut stops = None;
        let mut shapes = None;

        for idx in 0..archive.len() {
            let file = archive.by_index(idx)?;

            match file.name() {
                "calendar.txt" => calendar = Some(idx),
                "calendar_dates.txt" => calendar_dates = Some(idx),
                "routes.txt" => routes = Some(idx),
                "trips.txt" => trips = Some(idx),
                "stop_times.txt" => stop_times = Some(idx),
                "stops.txt" => stops = Some(idx),
                "shapes.txt" => shapes = Some(idx),
                _ => continue,
            }
        }

        Ok(Self {
            archive,
            calendar_idx: calendar.ok_or(err::GtfsErr::MissingFile("calendar.txt"))?,
            calendar_dates_idx: calendar_dates
                .ok_or(err::GtfsErr::MissingFile("calendar_dates.txt"))?,
            routes_idx: routes.ok_or(err::GtfsErr::MissingFile("routes.txt"))?,
            trips_idx: trips.ok_or(err::GtfsErr::MissingFile("trips.txt"))?,
            stop_times_idx: stop_times.ok_or(err::GtfsErr::MissingFile("stop_times.txt"))?,
            stops_idx: stops.ok_or(err::GtfsErr::MissingFile("stops.txt"))?,
            shapes_idx: shapes.ok_or(err::GtfsErr::MissingFile("shapes.txt"))?,
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

    pub fn routes(&mut self) -> Result<Routes<Cursor<Vec<u8>>>, GtfsErr> {
        Ok(Routes::new(self.reader_from_file(self.routes_idx)?))
    }

    pub fn trips(&mut self) -> Result<Trips<Cursor<Vec<u8>>>, GtfsErr> {
        Ok(Trips::new(self.reader_from_file(self.trips_idx)?))
    }

    pub fn calendars(&mut self) -> Result<Calendars<Cursor<Vec<u8>>>, GtfsErr> {
        Ok(Calendars::new(self.reader_from_file(self.calendar_idx)?))
    }

    pub fn calendar_dates(&mut self) -> Result<CalendarDates<Cursor<Vec<u8>>>, GtfsErr> {
        Ok(CalendarDates::new(
            self.reader_from_file(self.calendar_dates_idx)?,
        ))
    }

    pub fn shapes(&mut self) -> Result<Shapes<Cursor<Vec<u8>>>, GtfsErr> {
        Ok(Shapes::new(self.reader_from_file(self.shapes_idx)?))
    }
}

#[cfg(test)]
mod gtfs {
    use super::*;

    #[test]
    fn parses_operational_data_from_the_bundled_archive() {
        let src = include_bytes!("../../google_transit.zip");
        let mut schedule = GtfsSchedule::new(src).unwrap();

        assert_eq!(schedule.routes().unwrap().count(), 1);
        assert_eq!(schedule.calendars().unwrap().count(), 4);
        assert_eq!(schedule.calendar_dates().unwrap().count(), 16);
        assert_eq!(schedule.stops().unwrap().count(), 191);
        assert_eq!(schedule.trips().unwrap().count(), 6738);
        assert_eq!(schedule.shapes().unwrap().count(), 18_362);
        assert!(schedule.stop_times().unwrap().next().unwrap().is_ok());
    }
}
