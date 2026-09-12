pub struct Store {
    handle: (),
}

impl Store {
    pub fn ingestion(&self) {
        // TODO:
        // 0. fetch google_transit.zip
        // 1. open new db
        // 2. create iritsi_gtfs::GtfsSchedule::new() (in another thread?)
        // 3. iterate over stops, stop_times, routes, trips, calendars, calendar_dates and shapes
        // 4. insert 25-50k entries at a time
    }
}
