use std::path::Path;

use iritsi_gtfs::{GtfsSchedule, LocationType, ServiceException, StopId};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{Departure, Line, Station, date::ServiceDay, err::CoreErr};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS stops (
    id INTEGER PRIMARY KEY,
    gtfs_id TEXT NOT NULL,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    zone_id TEXT NOT NULL,
    location_type INTEGER NOT NULL,
    parent_station INTEGER
);
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
    stop_id INTEGER NOT NULL,
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
CREATE TABLE IF NOT EXISTS station_lines (
    station_id INTEGER NOT NULL,
    line INTEGER NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY (station_id, line)
) WITHOUT ROWID;
";

pub const META_FEED_HASH: &str = "feed_hash";
pub const META_LAST_CHECK: &str = "last_check";
pub const META_IMPORTED_AT: &str = "imported_at";
pub const META_SOURCE: &str = "source";
pub const META_DOWNLOAD_MS: &str = "download_ms";
pub const META_IMPORT_MS: &str = "import_ms";
pub const META_FEED_SIZE: &str = "feed_size";

const SCHEMA_VERSION: i64 = 2;

const INDEXES: &str = "
CREATE INDEX IF NOT EXISTS stop_times_by_stop ON stop_times (stop_id, departure_time);
CREATE INDEX IF NOT EXISTS stops_by_parent ON stops (parent_station);
CREATE INDEX IF NOT EXISTS calendar_dates_by_date ON calendar_dates (date);
";

const USER_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS user.favorites (
    station_id TEXT PRIMARY KEY,
    added_at INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS user.settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
";

const STATION_SELECT: &str = "
SELECT s.id, s.name, s.zone_id, s.lat, s.lon,
    (SELECT group_concat(line) FROM (
        SELECT line FROM station_lines l WHERE l.station_id = s.id ORDER BY line
    )),
    f.station_id IS NOT NULL
FROM stops s
LEFT JOIN user.favorites f ON f.station_id = s.gtfs_id
LEFT JOIN station_lines o ON o.station_id = s.id
    AND o.line = (SELECT MIN(line) FROM station_lines m WHERE m.station_id = s.id)
WHERE s.location_type = 1";

const STATION_ORDER: &str = "ORDER BY o.line IS NULL, o.line, o.position, s.name";

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path, user: &Path) -> Result<Self, CoreErr> {
        let mut conn = Connection::open(path)?;
        if !is_current(&conn)? {
            drop(conn);
            std::fs::remove_file(path)?;
            conn = Connection::open(path)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        conn.execute_batch(SCHEMA)?;
        conn.execute_batch(INDEXES)?;
        conn.execute("ATTACH DATABASE ?1 AS user", [user.to_string_lossy()])?;
        conn.execute_batch(USER_SCHEMA)?;
        Ok(Self { conn })
    }

    #[cfg(test)]
    pub fn conn(&self) -> &Connection {
        &self.conn
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
        conn.execute_batch(
            "PRAGMA journal_mode = OFF;
            PRAGMA synchronous = OFF;
            PRAGMA locking_mode = EXCLUSIVE;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -65536;",
        )?;
        conn.execute_batch(SCHEMA)?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;

        let tx = conn.transaction()?;
        for &(key, value) in meta {
            set_meta(&tx, key, value)?;
        }
        {
            let mut insert =
                tx.prepare("INSERT INTO stops VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")?;
            for stop in schedule.stops()? {
                let stop = stop?;
                let location_type = match stop.location_type {
                    LocationType::Platform => 0,
                    LocationType::Station => 1,
                    LocationType::Entrance => 2,
                };
                insert.execute(params![
                    key(stop.id),
                    stop.id.to_string(),
                    stop.code,
                    stop.name,
                    stop.lat,
                    stop.lon,
                    stop.zone_id,
                    location_type,
                    stop.parent_station.map(key),
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
                    key(stop_time.stop_id),
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
        for line in Line::ALL {
            insert_line(&tx, line)?;
        }
        tx.execute_batch(INDEXES)?;
        tx.commit()?;
        conn.execute_batch("PRAGMA analysis_limit = 400; ANALYZE;")?;
        Ok(())
    }

    pub fn search_stations(&self, query: &str) -> Result<Vec<Station>, CoreErr> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "{STATION_SELECT} AND s.name LIKE '%' || ?1 || '%' ESCAPE '\\' {STATION_ORDER}"
        ))?;
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
            .prepare_cached(&format!("{STATION_SELECT} AND s.id = ?1"))?
            .query_row([key(id)], Station::from_row)
            .optional()?)
    }

    pub fn timetable_stats(&self) -> Result<(u32, u32, u32, Option<u32>, Option<u32>), CoreErr> {
        Ok(self.conn.query_row(
            "SELECT
                (SELECT COUNT(*) FROM stops WHERE location_type = 1),
                (SELECT COUNT(*) FROM trips),
                (SELECT COUNT(*) FROM stop_times),
                (SELECT MIN(d) FROM (
                    SELECT start_date AS d FROM calendar
                    UNION ALL SELECT date FROM calendar_dates WHERE exception = 1
                )),
                (SELECT MAX(d) FROM (
                    SELECT end_date AS d FROM calendar
                    UNION ALL SELECT date FROM calendar_dates WHERE exception = 1
                ))",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?)
    }

    pub fn favorites(&self) -> Result<Vec<Station>, CoreErr> {
        self.conn
            .prepare_cached(&format!(
                "{STATION_SELECT} AND f.station_id IS NOT NULL ORDER BY f.added_at, s.name"
            ))?
            .query_map([], Station::from_row)?
            .map(|row| Ok(row?))
            .collect()
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, CoreErr> {
        Ok(self
            .conn
            .prepare_cached("SELECT value FROM user.settings WHERE key = ?1")?
            .query_row([key], |row| row.get(0))
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: Option<&str>) -> Result<(), CoreErr> {
        match value {
            Some(value) => self
                .conn
                .prepare_cached("INSERT OR REPLACE INTO user.settings VALUES (?1, ?2)")?
                .execute([key, value])?,
            None => self
                .conn
                .prepare_cached("DELETE FROM user.settings WHERE key = ?1")?
                .execute([key])?,
        };
        Ok(())
    }

    pub fn clear_settings(&self) -> Result<(), CoreErr> {
        self.conn.execute("DELETE FROM user.settings", [])?;
        Ok(())
    }

    pub fn clear_favorites(&self) -> Result<(), CoreErr> {
        self.conn.execute("DELETE FROM user.favorites", [])?;
        Ok(())
    }

    pub fn set_favorite(&self, id: StopId, favorite: bool, now: i64) -> Result<(), CoreErr> {
        if favorite {
            self.conn
                .prepare_cached("INSERT OR IGNORE INTO user.favorites VALUES (?1, ?2)")?
                .execute(params![id.to_string(), now])?;
        } else {
            self.conn
                .prepare_cached("DELETE FROM user.favorites WHERE station_id = ?1")?
                .execute([id.to_string()])?;
        }
        Ok(())
    }

    pub fn departures(
        &self,
        station: StopId,
        day: ServiceDay,
        after: i64,
        limit: u32,
    ) -> Result<Vec<Departure>, CoreErr> {
        let start = day.start();
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
            params![key(station), day.date(), day.weekday(), after.max(0), limit],
            |row| {
                Ok(Departure {
                    trip_id: row.get(0)?,
                    departs_at: start + row.get::<_, i64>(1)?,
                    platform: from_key(row.get(2)?),
                    headsign: row.get(3)?,
                    direction_id: row.get(4)?,
                })
            },
        )?
        .map(|row| Ok(row?))
        .collect()
    }
}

fn insert_line(conn: &Connection, line: Line) -> Result<(), CoreErr> {
    let (from, to) = line.termini();
    let trip: Option<u32> = conn
        .query_row(
            "WITH ends AS (
                SELECT trip_id, MIN(stop_seq) AS first, MAX(stop_seq) AS last
                FROM stop_times GROUP BY trip_id
            )
            SELECT e.trip_id FROM ends e
            JOIN stop_times a ON a.trip_id = e.trip_id AND a.stop_seq = e.first
            JOIN stops pa ON pa.id = a.stop_id
            JOIN stops sa ON sa.id = pa.parent_station
            JOIN stop_times b ON b.trip_id = e.trip_id AND b.stop_seq = e.last
            JOIN stops pb ON pb.id = b.stop_id
            JOIN stops sb ON sb.id = pb.parent_station
            WHERE sa.name = ?1 AND sb.name = ?2
            LIMIT 1",
            [from, to],
            |row| row.get(0),
        )
        .optional()?;
    let Some(trip) = trip else {
        return Ok(());
    };
    conn.execute(
        "INSERT OR IGNORE INTO station_lines
        SELECT p.parent_station, ?2, ROW_NUMBER() OVER (ORDER BY st.stop_seq)
        FROM stop_times st JOIN stops p ON p.id = st.stop_id
        WHERE st.trip_id = ?1",
        params![trip, line as u8],
    )?;
    Ok(())
}

fn set_meta(conn: &Connection, key: &str, value: i64) -> Result<(), CoreErr> {
    conn.prepare_cached("INSERT OR REPLACE INTO meta VALUES (?1, ?2)")?
        .execute(params![key, value])?;
    Ok(())
}

fn is_current(conn: &Connection) -> Result<bool, CoreErr> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let tables: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_schema", [], |row| row.get(0))?;
    Ok(version == SCHEMA_VERSION || tables == 0)
}

fn key(id: StopId) -> i64 {
    id.station as i64 * 1000 + id.child.map_or(999, i64::from)
}

fn from_key(key: i64) -> StopId {
    StopId {
        station: (key / 1000) as u16,
        child: match key % 1000 {
            999 => None,
            child => Some(child as u8),
        },
    }
}

impl Station {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: from_key(row.get(0)?),
            name: row.get(1)?,
            zone_id: row.get(2)?,
            lat: row.get(3)?,
            lon: row.get(4)?,
            lines: row
                .get::<_, Option<String>>(5)?
                .unwrap_or_default()
                .split(',')
                .filter_map(Line::from_id)
                .collect(),
            favorite: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_keys_roundtrip() {
        for id in ["40", "40.0", "40.3", "1", "51.255"] {
            let id: StopId = id.parse().unwrap();
            assert_eq!(from_key(key(id)), id);
        }
        assert_eq!(key("40".parse().unwrap()), 40_999);
        assert_eq!(key("40.0".parse().unwrap()), 40_000);
    }

    #[test]
    fn replaces_timetables_of_older_schemas() {
        let dir = std::env::temp_dir().join(format!("iritsi-store-schema-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("iritsi.db");
        let old = Connection::open(&path).unwrap();
        old.execute_batch(
            "CREATE TABLE stops (id TEXT PRIMARY KEY); INSERT INTO stops VALUES ('1');",
        )
        .unwrap();
        drop(old);

        let store = Store::open(&path, &dir.join("user.db")).unwrap();
        let version: i64 = store
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(store.search_stations("").unwrap().is_empty());

        std::fs::remove_dir_all(dir).unwrap();
    }
}
