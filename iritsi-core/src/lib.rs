use std::path::{Path, PathBuf};

pub use err::CoreErr;
pub use iritsi_gtfs::StopId;

mod date;
mod err;
///! file system abstraction for atomic storage updates
mod fs;
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
    pub departure_time: u32,
    pub platform: StopId,
    pub headsign: String,
    pub direction_id: u8,
}

pub struct Iritsi {
    path: PathBuf,
    store: store::Store,
}

impl Iritsi {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, CoreErr> {
        let path = path.into();
        let store = store::Store::open(&path)?;
        Ok(Self { path, store })
    }

    pub fn ingest(&mut self, gtfs: &[u8]) -> Result<(), CoreErr> {
        fs::atomic_replace(&self.path, |tmp: &Path| store::Store::ingest(tmp, gtfs))?;
        self.store = store::Store::open(&self.path)?;
        Ok(())
    }

    pub fn search_stations(&self, query: &str) -> Result<Vec<Station>, CoreErr> {
        self.store.search_stations(query)
    }

    pub fn station(&self, id: StopId) -> Result<Option<Station>, CoreErr> {
        self.store.station(id)
    }

    pub fn departures(
        &self,
        station: StopId,
        date: u32,
        after: u32,
        limit: u32,
    ) -> Result<Vec<Departure>, CoreErr> {
        self.store.departures(station, date, after, limit)
    }
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
        let mut iritsi = Iritsi::open(dir.join("iritsi.db")).unwrap();
        iritsi.ingest(GTFS).unwrap();
        (iritsi, dir)
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
    fn lists_upcoming_departures() {
        let (iritsi, dir) = ingested();
        let santurtzi = iritsi.search_stations("Santurtzi").unwrap()[0].id;

        let departures = iritsi
            .departures(santurtzi, 20260917, 8 * 3600, 10)
            .unwrap();
        assert_eq!(departures.len(), 10);
        assert!(
            departures
                .windows(2)
                .all(|w| w[0].departure_time <= w[1].departure_time)
        );
        assert!(departures.iter().all(|d| d.departure_time >= 8 * 3600));
        assert!(
            departures
                .iter()
                .all(|d| d.platform.station == santurtzi.station)
        );

        assert!(
            iritsi
                .departures(santurtzi, 20300101, 0, 10)
                .unwrap()
                .is_empty()
        );
        assert!(iritsi.departures(santurtzi, 20260230, 0, 10).is_err());

        std::fs::remove_dir_all(dir).unwrap();
    }
}
