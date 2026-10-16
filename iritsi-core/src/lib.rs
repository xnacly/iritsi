use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::{Duration, SystemTime},
};

use date::ServiceDay;
pub use date::{Date, LocalTime};
pub use err::CoreErr;
pub use iritsi_gtfs::StopId;
pub use remote::FEED_URL;
use store::{META_FEED_HASH, META_LAST_CHECK};

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

#[cfg(target_os = "android")]
pub const ANDROID_APP_ID: &str = "me.xnacly.iritsi";

mod date;
mod err;
///! file system abstraction for atomic storage updates
mod fs;
mod remote;
///! sqlite backed data store engine
mod store;

#[derive(Debug, Clone, PartialEq)]
pub struct Station {
    pub id: StopId,
    pub name: String,
    pub zone_id: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Departure {
    pub trip_id: u32,
    pub departs_at: i64,
    pub platform: StopId,
    pub headsign: String,
    pub direction_id: u8,
}

impl Departure {
    pub fn local_time(&self) -> (u32, u32) {
        date::madrid_clock(self.departs_at)
    }

    pub fn minutes_until(&self) -> i64 {
        (self.departs_at - unix_now()).div_euclid(60)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    Skipped,
    Unchanged,
    Updated,
}

pub struct Iritsi {
    path: PathBuf,
    store: Mutex<store::Store>,
    refreshing: Mutex<()>,
}

impl Iritsi {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, CoreErr> {
        let path = path.into();
        let store = store::Store::open(&path)?;
        let iritsi = Self {
            path,
            store: Mutex::new(store),
            refreshing: Mutex::new(()),
        };
        #[cfg(feature = "bundled-feed")]
        if !iritsi.has_data()? {
            iritsi.ingest(remote::BUNDLED_FEED)?;
        }
        Ok(iritsi)
    }

    pub fn open_default() -> Result<Self, CoreErr> {
        let path = default_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::open(path)
    }

    fn store(&self) -> MutexGuard<'_, store::Store> {
        self.store.lock().unwrap_or_else(|err| err.into_inner())
    }

    pub fn has_data(&self) -> Result<bool, CoreErr> {
        Ok(self.store().meta(META_FEED_HASH)?.is_some())
    }

    pub fn refresh(&self) -> Result<Refresh, CoreErr> {
        let now = unix_now();
        if let Some(last_check) = self.store().meta(META_LAST_CHECK)?
            && now.saturating_sub(last_check) < REFRESH_INTERVAL.as_secs() as i64
        {
            return Ok(Refresh::Skipped);
        }
        self.force_refresh()
    }

    pub fn force_refresh(&self) -> Result<Refresh, CoreErr> {
        let feed = remote::fetch(FEED_URL, |_, _| {})?;
        self.apply_feed(&feed, unix_now())
    }

    fn apply_feed(&self, feed: &[u8], now: i64) -> Result<Refresh, CoreErr> {
        let _refreshing = self
            .refreshing
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let hash = remote::hash(feed) as i64;
        if self.store().meta(META_FEED_HASH)? == Some(hash) {
            self.store().set_meta(META_LAST_CHECK, now)?;
            return Ok(Refresh::Unchanged);
        }
        self.replace(feed, &[(META_FEED_HASH, hash), (META_LAST_CHECK, now)])?;
        Ok(Refresh::Updated)
    }

    pub fn ingest(&self, gtfs: &[u8]) -> Result<(), CoreErr> {
        let _refreshing = self
            .refreshing
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        self.replace(gtfs, &[(META_FEED_HASH, remote::hash(gtfs) as i64)])
    }

    fn replace(&self, gtfs: &[u8], meta: &[(&str, i64)]) -> Result<(), CoreErr> {
        fs::atomic_replace(&self.path, |tmp: &Path| {
            store::Store::ingest(tmp, gtfs, meta)
        })?;
        *self.store() = store::Store::open(&self.path)?;
        Ok(())
    }

    pub fn stations(&self) -> Result<Vec<Station>, CoreErr> {
        self.search_stations("")
    }

    pub fn search_stations(&self, query: &str) -> Result<Vec<Station>, CoreErr> {
        self.store().search_stations(query)
    }

    pub fn station(&self, id: StopId) -> Result<Option<Station>, CoreErr> {
        self.store().station(id)
    }

    pub fn departures(&self, station: StopId, limit: u32) -> Result<Vec<Departure>, CoreErr> {
        self.departures_at(station, unix_now(), limit)
    }

    pub fn departures_at(
        &self,
        station: StopId,
        unix: i64,
        limit: u32,
    ) -> Result<Vec<Departure>, CoreErr> {
        let today = ServiceDay::containing(unix);
        let yesterday = today.previous();
        let store = self.store();
        let mut departures =
            store.departures(station, yesterday, unix - yesterday.start(), limit)?;
        departures.extend(store.departures(station, today, unix - today.start(), limit)?);
        departures.sort_by_key(|d| d.departs_at);
        departures.truncate(limit as usize);
        Ok(departures)
    }
}

#[cfg(target_os = "android")]
fn default_path() -> Result<PathBuf, CoreErr> {
    Ok(PathBuf::from(format!(
        "/data/data/{ANDROID_APP_ID}/files/iritsi.db"
    )))
}

#[cfg(not(target_os = "android"))]
fn default_path() -> Result<PathBuf, CoreErr> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or(CoreErr::NoDataDir)?;
    Ok(data.join("iritsi").join("iritsi.db"))
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GTFS: &[u8] = include_bytes!("../../google_transit.zip");

    fn ingested() -> (Iritsi, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "iritsi-core-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let iritsi = Iritsi::open(dir.join("iritsi.db")).unwrap();
        iritsi.ingest(GTFS).unwrap();
        (iritsi, dir)
    }

    #[test]
    fn skips_ingesting_an_unchanged_feed() {
        let (iritsi, dir) = ingested();
        assert!(iritsi.has_data().unwrap());
        assert_eq!(iritsi.store().meta(META_LAST_CHECK).unwrap(), None);

        let now = unix_now();
        assert_eq!(iritsi.apply_feed(GTFS, now).unwrap(), Refresh::Unchanged);
        assert_eq!(iritsi.store().meta(META_LAST_CHECK).unwrap(), Some(now));
        assert_eq!(iritsi.refresh().unwrap(), Refresh::Skipped);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(not(feature = "bundled-feed"))]
    fn empty_database_has_no_data() {
        let dir = std::env::temp_dir().join(format!("iritsi-core-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iritsi = Iritsi::open(dir.join("iritsi.db")).unwrap();
        assert!(!iritsi.has_data().unwrap());
        assert!(iritsi.search_stations("").unwrap().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(feature = "bundled-feed")]
    fn empty_database_is_seeded_from_the_bundled_feed() {
        let dir = std::env::temp_dir().join(format!("iritsi-core-seed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iritsi = Iritsi::open(dir.join("iritsi.db")).unwrap();
        assert!(iritsi.has_data().unwrap());
        assert_eq!(iritsi.stations().unwrap().len(), 42);
        assert_eq!(iritsi.store().meta(META_LAST_CHECK).unwrap(), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[ignore]
    fn downloads_the_live_feed() {
        let dir = std::env::temp_dir().join(format!("iritsi-core-live-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iritsi = Iritsi::open(dir.join("iritsi.db")).unwrap();

        assert_eq!(iritsi.refresh().unwrap(), Refresh::Updated);
        assert_eq!(iritsi.refresh().unwrap(), Refresh::Skipped);
        assert_eq!(iritsi.search_stations("").unwrap().len(), 42);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn searches_stations() {
        let (iritsi, dir) = ingested();

        let found = iritsi.search_stations("casco").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Zazpikaleak/Casco Viejo");
        assert_eq!(found[0].id, "6".parse().unwrap());

        assert_eq!(iritsi.search_stations("").unwrap().len(), 42);
        assert!(iritsi.search_stations("%").unwrap().is_empty());

        std::fs::remove_dir_all(dir).unwrap();
    }

    const HOUR: i64 = 60 * 60;
    const SEP_17_2026_UTC: i64 = 1_789_603_200;

    #[test]
    fn lists_upcoming_departures() {
        let (iritsi, dir) = ingested();
        let santurtzi = iritsi.search_stations("Santurtzi").unwrap()[0].id;

        let at = SEP_17_2026_UTC + 6 * HOUR;
        let departures = iritsi.departures_at(santurtzi, at, 10).unwrap();
        assert_eq!(departures.len(), 10);
        assert!(
            departures
                .windows(2)
                .all(|w| w[0].departs_at <= w[1].departs_at)
        );
        assert!(departures.iter().all(|d| d.departs_at >= at));
        assert!(departures.iter().all(|d| d.departs_at < at + HOUR));
        assert!(
            departures
                .iter()
                .all(|d| d.platform.station == santurtzi.station)
        );

        assert!(
            iritsi
                .departures_at(santurtzi, SEP_17_2026_UTC + 365 * 24 * HOUR, 10)
                .unwrap()
                .is_empty()
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saturday_night_service_runs_past_midnight() {
        let (iritsi, dir) = ingested();
        let casco = iritsi.search_stations("Casco").unwrap()[0].id;

        let sunday_3am_madrid = SEP_17_2026_UTC + 3 * 24 * HOUR + HOUR;
        let departures = iritsi.departures_at(casco, sunday_3am_madrid, 5).unwrap();
        assert_eq!(departures.len(), 5);
        assert!(departures.iter().all(|d| d.departs_at >= sunday_3am_madrid));
        assert!(departures[0].departs_at < sunday_3am_madrid + HOUR);

        std::fs::remove_dir_all(dir).unwrap();
    }
}
