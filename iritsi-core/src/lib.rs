use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime},
};

use date::ServiceDay;
pub use date::{Date, LocalTime};
pub use err::CoreErr;
pub use iritsi_gtfs::StopId;
pub use remote::FEED_URL;
pub use settings::{
    DEFAULT_ACCENT, DEFAULT_DEPARTURES, DEPARTURE_CHOICES, Settings, TimeDisplay, UpdateInterval,
};
use store::{
    META_DOWNLOAD_MS, META_FEED_HASH, META_FEED_SIZE, META_IMPORT_MS, META_IMPORTED_AT,
    META_LAST_CHECK, META_SOURCE,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_os = "android")]
pub const ANDROID_APP_ID: &str = "me.xnacly.iritsi";

mod date;
mod err;
///! file system abstraction for atomic storage updates
mod fs;
mod remote;
mod settings;
///! sqlite backed data store engine
mod store;

#[derive(Debug, Clone, PartialEq)]
pub struct Station {
    pub id: StopId,
    pub name: String,
    pub zone_id: String,
    pub lat: f64,
    pub lon: f64,
    pub lines: Vec<Line>,
    pub favorite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Line {
    L1 = 1,
    L2 = 2,
}

impl Line {
    pub const ALL: [Line; 2] = [Line::L1, Line::L2];

    pub fn name(self) -> &'static str {
        match self {
            Line::L1 => "L1",
            Line::L2 => "L2",
        }
    }

    pub fn termini(self) -> (&'static str, &'static str) {
        match self {
            Line::L1 => ("Etxebarri", "Plentzia"),
            Line::L2 => ("Basauri", "Kabiezes"),
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "1" => Some(Line::L1),
            "2" => Some(Line::L2),
            _ => None,
        }
    }
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
pub enum FeedSource {
    Bundled = 0,
    Downloaded = 1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimetableInfo {
    pub imported_at: Option<LocalTime>,
    pub checked_at: Option<LocalTime>,
    pub source: Option<FeedSource>,
    pub valid_from: Option<Date>,
    pub valid_until: Option<Date>,
    pub download_time: Option<Duration>,
    pub import_time: Option<Duration>,
    pub feed_size: Option<u64>,
    pub database_size: u64,
    pub user_database_size: u64,
    pub stations: u32,
    pub trips: u32,
    pub stop_times: u32,
}

impl TimetableInfo {
    pub fn update_time(&self) -> Option<Duration> {
        self.import_time
            .map(|import| import + self.download_time.unwrap_or_default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    Downloading { received: u64, total: Option<u64> },
    Importing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    Skipped,
    Unchanged,
    Updated,
}

pub struct Iritsi {
    path: PathBuf,
    user_path: PathBuf,
    store: Mutex<store::Store>,
    refreshing: Mutex<()>,
}

impl Iritsi {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, CoreErr> {
        let path = path.into();
        let user_path = path.with_extension("user.db");
        let store = store::Store::open(&path, &user_path)?;
        let iritsi = Self {
            path,
            user_path,
            store: Mutex::new(store),
            refreshing: Mutex::new(()),
        };
        #[cfg(feature = "bundled-feed")]
        if !iritsi.has_data()? {
            iritsi.ingest(remote::BUNDLED_FEED)?;
        }
        iritsi.backfill_import_meta()?;
        Ok(iritsi)
    }

    pub fn open_default() -> Result<Self, CoreErr> {
        let path = default_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::open(path)
    }

    fn backfill_import_meta(&self) -> Result<(), CoreErr> {
        let store = self.store();
        if store.meta(META_FEED_HASH)?.is_none() || store.meta(META_IMPORTED_AT)?.is_some() {
            return Ok(());
        }
        let last_check = store.meta(META_LAST_CHECK)?;
        let modified = std::fs::metadata(&self.path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|since| since.as_secs() as i64);
        if let Some(imported_at) = modified.or(last_check) {
            store.set_meta(META_IMPORTED_AT, imported_at)?;
        }
        let source = match last_check {
            Some(_) => FeedSource::Downloaded,
            None => FeedSource::Bundled,
        };
        store.set_meta(META_SOURCE, source as i64)?;
        Ok(())
    }

    fn store(&self) -> MutexGuard<'_, store::Store> {
        self.store.lock().unwrap_or_else(|err| err.into_inner())
    }

    pub fn has_data(&self) -> Result<bool, CoreErr> {
        Ok(self.store().meta(META_FEED_HASH)?.is_some())
    }

    pub fn refresh(&self) -> Result<Refresh, CoreErr> {
        self.refresh_with(|_| {})
    }

    pub fn refresh_with(&self, progress: impl FnMut(Progress)) -> Result<Refresh, CoreErr> {
        let now = unix_now();
        let last_check = self.store().meta(META_LAST_CHECK)?;
        let due = match self.settings()?.update_interval {
            UpdateInterval::Manual => !self.has_data()?,
            UpdateInterval::Every(every) => {
                last_check.is_none_or(|last| now.saturating_sub(last) >= every.as_secs() as i64)
            }
        };
        if !due {
            return Ok(Refresh::Skipped);
        }
        self.force_refresh_with(progress)
    }

    pub fn force_refresh(&self) -> Result<Refresh, CoreErr> {
        self.force_refresh_with(|_| {})
    }

    pub fn force_refresh_with(&self, progress: impl FnMut(Progress)) -> Result<Refresh, CoreErr> {
        self.download(false, progress)
    }

    pub fn redownload_timetable_with(
        &self,
        progress: impl FnMut(Progress),
    ) -> Result<Refresh, CoreErr> {
        self.download(true, progress)
    }

    fn download(
        &self,
        reimport: bool,
        mut progress: impl FnMut(Progress),
    ) -> Result<Refresh, CoreErr> {
        let started = Instant::now();
        let feed = remote::fetch(FEED_URL, |received, total| {
            progress(Progress::Downloading { received, total })
        })?;
        self.apply_feed(
            &feed,
            unix_now(),
            Some(started.elapsed()),
            reimport,
            progress,
        )
    }

    fn apply_feed(
        &self,
        feed: &[u8],
        now: i64,
        download_time: Option<Duration>,
        reimport: bool,
        mut progress: impl FnMut(Progress),
    ) -> Result<Refresh, CoreErr> {
        let _refreshing = self
            .refreshing
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let hash = remote::hash(feed) as i64;
        if !reimport && self.store().meta(META_FEED_HASH)? == Some(hash) {
            self.store().set_meta(META_LAST_CHECK, now)?;
            return Ok(Refresh::Unchanged);
        }
        progress(Progress::Importing);
        self.replace(
            feed,
            &[
                (META_FEED_HASH, hash),
                (META_LAST_CHECK, now),
                (META_IMPORTED_AT, now),
                (META_SOURCE, FeedSource::Downloaded as i64),
            ],
            download_time,
        )?;
        Ok(Refresh::Updated)
    }

    pub fn ingest(&self, gtfs: &[u8]) -> Result<(), CoreErr> {
        let _refreshing = self
            .refreshing
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        self.replace(
            gtfs,
            &[
                (META_FEED_HASH, remote::hash(gtfs) as i64),
                (META_IMPORTED_AT, unix_now()),
                (META_SOURCE, FeedSource::Bundled as i64),
            ],
            None,
        )
    }

    fn replace(
        &self,
        gtfs: &[u8],
        meta: &[(&str, i64)],
        download_time: Option<Duration>,
    ) -> Result<(), CoreErr> {
        let started = Instant::now();
        fs::atomic_replace(&self.path, |tmp: &Path| {
            store::Store::ingest(tmp, gtfs, meta)
        })?;
        let store = store::Store::open(&self.path, &self.user_path)?;
        store.set_meta(META_IMPORT_MS, started.elapsed().as_millis() as i64)?;
        store.set_meta(META_FEED_SIZE, gtfs.len() as i64)?;
        if let Some(download_time) = download_time {
            store.set_meta(META_DOWNLOAD_MS, download_time.as_millis() as i64)?;
        }
        *self.store() = store;
        Ok(())
    }

    pub fn settings(&self) -> Result<Settings, CoreErr> {
        let store = self.store();
        let mut values = Vec::new();
        for key in settings::KEYS {
            values.push((key, store.setting(key)?));
        }
        Ok(Settings::from_values(|key| {
            values
                .iter()
                .find(|(k, _)| *k == key)
                .and_then(|(_, v)| v.clone())
        }))
    }

    pub fn set_update_interval(&self, interval: UpdateInterval) -> Result<(), CoreErr> {
        self.store()
            .set_setting(settings::UPDATE_INTERVAL, Some(&interval.to_setting()))
    }

    pub fn set_accent(&self, accent: Option<&str>) -> Result<(), CoreErr> {
        let accent = accent.map(str::to_ascii_lowercase);
        if let Some(accent) = &accent
            && !settings::is_hex_color(accent)
        {
            return Err(CoreErr::InvalidColor(accent.clone()));
        }
        self.store()
            .set_setting(settings::ACCENT, accent.as_deref())
    }

    pub fn set_departures(&self, departures: u32) -> Result<(), CoreErr> {
        if !DEPARTURE_CHOICES.contains(&departures) {
            return Err(CoreErr::InvalidDepartures(departures));
        }
        self.store()
            .set_setting(settings::DEPARTURES, Some(&departures.to_string()))
    }

    pub fn set_time_display(&self, display: TimeDisplay) -> Result<(), CoreErr> {
        self.store()
            .set_setting(settings::TIME_DISPLAY, Some(display.name()))
    }

    pub fn reset_settings(&self) -> Result<(), CoreErr> {
        self.store().clear_settings()
    }

    pub fn clear_favorites(&self) -> Result<(), CoreErr> {
        self.store().clear_favorites()
    }

    pub fn timetable_info(&self) -> Result<TimetableInfo, CoreErr> {
        let store = self.store();
        let (stations, trips, stop_times, valid_from, valid_until) = store.timetable_stats()?;
        Ok(TimetableInfo {
            imported_at: store.meta(META_IMPORTED_AT)?.map(LocalTime::madrid),
            checked_at: store.meta(META_LAST_CHECK)?.map(LocalTime::madrid),
            source: store.meta(META_SOURCE)?.map(|source| match source {
                0 => FeedSource::Bundled,
                _ => FeedSource::Downloaded,
            }),
            valid_from: valid_from.map(Date::from_yyyymmdd),
            valid_until: valid_until.map(Date::from_yyyymmdd),
            download_time: store.meta(META_DOWNLOAD_MS)?.map(millis),
            import_time: store.meta(META_IMPORT_MS)?.map(millis),
            feed_size: store.meta(META_FEED_SIZE)?.map(|size| size as u64),
            database_size: file_size(&self.path),
            user_database_size: file_size(&self.user_path),
            stations,
            trips,
            stop_times,
        })
    }

    pub fn stations(&self) -> Result<Vec<Station>, CoreErr> {
        self.search_stations("")
    }

    pub fn search_stations(&self, query: &str) -> Result<Vec<Station>, CoreErr> {
        self.store().search_stations(query)
    }

    pub fn favorites(&self) -> Result<Vec<Station>, CoreErr> {
        self.store().favorites()
    }

    pub fn set_favorite(&self, id: StopId, favorite: bool) -> Result<(), CoreErr> {
        self.store().set_favorite(id, favorite, unix_now())
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

fn millis(ms: i64) -> Duration {
    Duration::from_millis(ms.max(0) as u64)
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map_or(0, |meta| meta.len())
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
    fn describes_the_timetable() {
        let (iritsi, dir) = ingested();
        let info = iritsi.timetable_info().unwrap();
        assert_eq!(info.source, Some(FeedSource::Bundled));
        assert!(info.imported_at.is_some());
        assert_eq!(info.checked_at, None);
        assert_eq!(info.valid_from.unwrap().to_string(), "2026-09-10");
        assert_eq!(info.valid_until.unwrap().to_string(), "2026-10-10");
        assert_eq!(
            (info.stations, info.trips, info.stop_times),
            (42, 6738, 173_557)
        );
        assert_eq!(info.download_time, None);
        assert!(info.import_time.is_some());
        assert_eq!(info.update_time(), info.import_time);
        assert_eq!(info.feed_size, Some(GTFS.len() as u64));
        assert!(info.database_size > 1024 * 1024);
        assert!(info.user_database_size > 0);

        iritsi
            .apply_feed(GTFS, unix_now(), None, false, |_| {})
            .unwrap();
        assert!(iritsi.timetable_info().unwrap().checked_at.is_some());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn backfills_import_meta_of_older_databases() {
        let (iritsi, dir) = ingested();
        iritsi
            .apply_feed(GTFS, unix_now(), None, false, |_| {})
            .unwrap();
        iritsi
            .store()
            .conn()
            .execute(
                "DELETE FROM meta WHERE key IN ('imported_at', 'source')",
                [],
            )
            .unwrap();
        drop(iritsi);

        let info = Iritsi::open(dir.join("iritsi.db"))
            .unwrap()
            .timetable_info()
            .unwrap();
        assert!(info.imported_at.is_some());
        assert_eq!(info.source, Some(FeedSource::Downloaded));

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stores_settings_next_to_favorites() {
        let (iritsi, dir) = ingested();
        assert_eq!(iritsi.settings().unwrap(), Settings::default());

        iritsi.set_update_interval(UpdateInterval::Manual).unwrap();
        iritsi.set_accent(Some("#00AAFF")).unwrap();
        assert!(iritsi.set_accent(Some("blue")).is_err());
        iritsi.ingest(GTFS).unwrap();

        let settings = iritsi.settings().unwrap();
        assert_eq!(settings.update_interval, UpdateInterval::Manual);
        assert_eq!(settings.accent, "#00aaff");
        assert_eq!(iritsi.refresh().unwrap(), Refresh::Skipped);

        iritsi.set_accent(None).unwrap();
        assert_eq!(iritsi.settings().unwrap().accent, DEFAULT_ACCENT);

        iritsi.set_departures(30).unwrap();
        assert!(iritsi.set_departures(7).is_err());
        iritsi.set_time_display(TimeDisplay::Clock).unwrap();
        let settings = iritsi.settings().unwrap();
        assert_eq!(
            (settings.departures, settings.time_display),
            (30, TimeDisplay::Clock)
        );

        let abando = iritsi.search_stations("Abando").unwrap()[0].id;
        iritsi.set_favorite(abando, true).unwrap();
        iritsi.reset_settings().unwrap();
        assert_eq!(iritsi.settings().unwrap(), Settings::default());
        assert_eq!(iritsi.favorites().unwrap().len(), 1);
        iritsi.clear_favorites().unwrap();
        assert!(iritsi.favorites().unwrap().is_empty());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reimports_unchanged_feeds_when_asked() {
        let (iritsi, dir) = ingested();
        let before = iritsi.timetable_info().unwrap().imported_at;
        let mut phases = Vec::new();
        assert_eq!(
            iritsi
                .apply_feed(GTFS, unix_now() + 120, None, true, |p| phases.push(p))
                .unwrap(),
            Refresh::Updated
        );
        assert_eq!(phases, vec![Progress::Importing]);
        assert_ne!(iritsi.timetable_info().unwrap().imported_at, before);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reports_importing_only_for_changed_feeds() {
        let (iritsi, dir) = ingested();
        let mut phases = Vec::new();
        iritsi
            .apply_feed(GTFS, unix_now(), None, false, |p| phases.push(p))
            .unwrap();
        assert!(phases.is_empty());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn skips_ingesting_an_unchanged_feed() {
        let (iritsi, dir) = ingested();
        assert!(iritsi.has_data().unwrap());
        assert_eq!(iritsi.store().meta(META_LAST_CHECK).unwrap(), None);

        let now = unix_now();
        assert_eq!(
            iritsi.apply_feed(GTFS, now, None, false, |_| {}).unwrap(),
            Refresh::Unchanged
        );
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

        let mut phases = Vec::new();
        assert_eq!(
            iritsi.refresh_with(|p| phases.push(p)).unwrap(),
            Refresh::Updated
        );
        let info = iritsi.timetable_info().unwrap();
        assert!(info.download_time.is_some());
        assert!(info.update_time().unwrap() > info.import_time.unwrap());
        assert!(matches!(
            phases.first(),
            Some(Progress::Downloading { received: 0, .. })
        ));
        assert_eq!(phases.last(), Some(&Progress::Importing));
        let Some(Progress::Downloading { received, total }) = phases.iter().rev().nth(1) else {
            panic!("no download progress");
        };
        assert_eq!(Some(*received), *total);
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

    #[test]
    fn orders_stations_along_the_lines() {
        let (iritsi, dir) = ingested();
        let stations = iritsi.stations().unwrap();
        let names: Vec<_> = stations.iter().map(|s| s.name.as_str()).collect();

        assert_eq!(stations.len(), 42);
        assert_eq!(names[0], "Etxebarri");
        assert_eq!(names[28], "Plentzia");
        assert_eq!(names[29], "Basauri");
        assert_eq!(names[41], "Kabiezes");

        let lines = |name| {
            stations
                .iter()
                .find(|s| s.name == name)
                .unwrap()
                .lines
                .clone()
        };
        assert_eq!(lines("Abando"), vec![Line::L1, Line::L2]);
        assert_eq!(lines("Plentzia"), vec![Line::L1]);
        assert_eq!(lines("Santurtzi"), vec![Line::L2]);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn favorites_survive_feed_updates() {
        let (iritsi, dir) = ingested();
        let abando = iritsi.search_stations("Abando").unwrap()[0].id;
        let ariz = iritsi.search_stations("Ariz").unwrap()[0].id;

        iritsi.set_favorite(ariz, true).unwrap();
        iritsi.set_favorite(abando, true).unwrap();
        iritsi.set_favorite(abando, true).unwrap();
        assert!(iritsi.station(abando).unwrap().unwrap().favorite);

        iritsi
            .apply_feed(b"not a feed", 0, None, false, |_| {})
            .unwrap_err();
        iritsi.ingest(GTFS).unwrap();

        let favorites: Vec<_> = iritsi
            .favorites()
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(favorites.len(), 2);
        assert!(favorites.contains(&abando) && favorites.contains(&ariz));

        iritsi.set_favorite(ariz, false).unwrap();
        assert_eq!(iritsi.favorites().unwrap().len(), 1);
        assert!(!iritsi.station(ariz).unwrap().unwrap().favorite);

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
