use std::path::Path;

use iritsi_gtfs::{GtfsErr, GtfsSchedule, LocationType, ServiceException, StopId};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{Departure, Station, err::CoreErr};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS stops (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    zone_id TEXT NOT NULL,
    location_type INTEGER NOT NULL,
    parent_station TEXT
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS routes (
    id TEXT PRIMARY KEY,
    short_name TEXT NOT NULL,
    long_name TEXT NOT NULL,
    route_type INTEGER NOT NULL,
    color TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS trips (
    id INTEGER PRIMARY KEY,
    route_id TEXT NOT NULL,
    service_id TEXT NOT NULL,
    headsign TEXT NOT NULL,
    direction_id INTEGER NOT NULL,
    shape_id TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS stop_times (
    trip_id INTEGER NOT NULL,
    stop_seq INTEGER NOT NULL,
    arrival_time INTEGER NOT NULL,
    departure_time INTEGER NOT NULL,
    stop_id TEXT NOT NULL,
    PRIMARY KEY (trip_id, stop_seq)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS calendar (
    service_id TEXT PRIMARY KEY,
    weekdays INTEGER NOT NULL,
    start_date INTEGER NOT NULL,
    end_date INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS calendar_dates (
    service_id TEXT NOT NULL,
    date INTEGER NOT NULL,
    exception INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS shapes (
    shape_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    distance_traveled REAL NOT NULL,
    PRIMARY KEY (shape_id, sequence)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value INTEGER NOT NULL
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS stop_times_by_stop ON stop_times (stop_id, departure_time);
CREATE INDEX IF NOT EXISTS stops_by_parent ON stops (parent_station);
CREATE INDEX IF NOT EXISTS calendar_dates_by_date ON calendar_dates (date);
";

pub const META_FEED_HASH: &str = "feed_hash";
pub const META_LAST_CHECK: &str = "last_check";

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, CoreErr> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn meta(&self, key: &str) -> Result<Option<i64>, CoreErr> {
        Ok(self
            .conn
            .prepare_cached("SELECT value FROM meta WHERE key = ?1")?
            .query_row([key], |row| row.get(0))
            .optional()?)
    }

    pub fn set_meta(&self, key: &str, value: i64) -> Result<(), CoreErr> {
        set_meta(&self.conn, key, value)
    }

    pub fn ingest(path: &Path, gtfs: &[u8], meta: &[(&str, i64)]) -> Result<(), CoreErr> {
        let mut schedule = GtfsSchedule::new(gtfs)?;
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")?;
        conn.execute_batch(SCHEMA)?;

        let tx = conn.transaction()?;
        for &(key, value) in meta {
            set_meta(&tx, key, value)?;
        }
        {
            let mut insert =
                tx.prepare("INSERT INTO stops VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")?;
            for stop in schedule.stops()? {
                let stop = stop?;
                let location_type = match stop.location_type {
                    LocationType::Platform => 0,
                    LocationType::Station => 1,
                    LocationType::Entrance => 2,
                };
                insert.execute(params![
                    stop.id.to_string(),
                    stop.code,
                    stop.name,
                    stop.lat,
                    stop.lon,
                    stop.zone_id,
                    location_type,
                    stop.parent_station.map(|id| id.to_string()),
                ])?;
            }

            let mut insert = tx.prepare("INSERT INTO routes VALUES (?1, ?2, ?3, ?4, ?5)")?;
            for route in schedule.routes()? {
                let route = route?;
                insert.execute(params![
                    route.id,
                    route.short_name,
                    route.long_name,
                    route.route_type,
                    route.color
                ])?;
            }

            let mut insert = tx.prepare("INSERT INTO trips VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
            for trip in schedule.trips()? {
                let trip = trip?;
                insert.execute(params![
                    trip.id,
                    trip.route_id,
                    trip.service_id,
                    trip.headsign,
                    trip.direction_id,
                    trip.shape_id
                ])?;
            }

            let mut insert = tx.prepare("INSERT INTO stop_times VALUES (?1, ?2, ?3, ?4, ?5)")?;
            for stop_time in schedule.stop_times()? {
                let stop_time = stop_time?;
                insert.execute(params![
                    stop_time.trip_id,
                    stop_time.stop_seq,
                    stop_time.arrival_time,
                    stop_time.departure_time,
                    stop_time.stop_id.to_string(),
                ])?;
            }

            let mut insert = tx.prepare("INSERT INTO calendar VALUES (?1, ?2, ?3, ?4)")?;
            for calendar in schedule.calendars()? {
                let calendar = calendar?;
                let weekdays = calendar
                    .weekdays
                    .iter()
                    .enumerate()
                    .fold(0u8, |mask, (day, &active)| mask | (active as u8) << day);
                insert.execute(params![
                    calendar.service_id,
                    weekdays,
                    calendar.start_date,
                    calendar.end_date
                ])?;
            }

            let mut insert = tx.prepare("INSERT INTO calendar_dates VALUES (?1, ?2, ?3)")?;
            for date in schedule.calendar_dates()? {
                let date = date?;
                let exception = match date.exception {
                    ServiceException::Added => 1,
                    ServiceException::Removed => 2,
                };
                insert.execute(params![date.service_id, date.date, exception])?;
            }

            let mut insert = tx.prepare("INSERT INTO shapes VALUES (?1, ?2, ?3, ?4, ?5)")?;
            for point in schedule.shapes()? {
                let point = point?;
                insert.execute(params![
                    point.shape_id,
                    point.sequence,
                    point.lat,
                    point.lon,
                    point.distance_traveled
                ])?;
            }
        }
        tx.commit()?;
        conn.execute_batch("ANALYZE;")?;
        Ok(())
    }

    pub fn search_stations(&self, query: &str) -> Result<Vec<Station>, CoreErr> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name, zone_id, lat, lon FROM stops
             WHERE location_type = 1 AND name LIKE '%' || ?1 || '%' ESCAPE '\\'
             ORDER BY name",
        )?;
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        stmt.query_map([escaped], Station::from_row)?
            .map(|row| Ok(row?))
            .collect()
    }

    pub fn station(&self, id: StopId) -> Result<Option<Station>, CoreErr> {
        Ok(self
            .conn
            .prepare_cached(
                "SELECT id, name, zone_id, lat, lon FROM stops WHERE location_type = 1 AND id = ?1",
            )?
            .query_row([id.to_string()], Station::from_row)
            .optional()?)
    }

    pub fn departures(
        &self,
        station: StopId,
        date: u32,
        after: u32,
        limit: u32,
    ) -> Result<Vec<Departure>, CoreErr> {
        let weekday = crate::date::weekday(date).ok_or(CoreErr::InvalidDate(date))?;
        let mut stmt = self.conn.prepare_cached(
            "WITH active(service_id) AS (
                SELECT service_id FROM calendar
                WHERE ?2 BETWEEN start_date AND end_date AND weekdays & (1 << ?3) != 0
                UNION
                SELECT service_id FROM calendar_dates WHERE date = ?2 AND exception = 1
                EXCEPT
                SELECT service_id FROM calendar_dates WHERE date = ?2 AND exception = 2
            )
            SELECT st.trip_id, st.departure_time, st.stop_id, t.headsign, t.direction_id
            FROM stops p
            JOIN stop_times st ON st.stop_id = p.id
            JOIN trips t ON t.id = st.trip_id
            WHERE p.parent_station = ?1
              AND st.departure_time >= ?4
              AND t.service_id IN active
              AND EXISTS (
                SELECT 1 FROM stop_times n WHERE n.trip_id = st.trip_id AND n.stop_seq > st.stop_seq
              )
            ORDER BY st.departure_time
            LIMIT ?5",
        )?;
        stmt.query_map(
            params![station.to_string(), date, weekday, after, limit],
            |row| {
                Ok(Departure {
                    trip_id: row.get(0)?,
                    departure_time: row.get(1)?,
                    platform: parse_id(row.get_ref(2)?.as_str()?)?,
                    headsign: row.get(3)?,
                    direction_id: row.get(4)?,
                })
            },
        )?
        .map(|row| Ok(row?))
        .collect()
    }
}

fn set_meta(conn: &Connection, key: &str, value: i64) -> Result<(), CoreErr> {
    conn.prepare_cached("INSERT OR REPLACE INTO meta VALUES (?1, ?2)")?
        .execute(params![key, value])?;
    Ok(())
}

fn parse_id(id: &str) -> rusqlite::Result<StopId> {
    id.parse().map_err(|err: GtfsErr| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            format!("{err:?}").into(),
        )
    })
}

impl Station {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: parse_id(row.get_ref(0)?.as_str()?)?,
            name: row.get(1)?,
            zone_id: row.get(2)?,
            lat: row.get(3)?,
            lon: row.get(4)?,
        })
    }
}
