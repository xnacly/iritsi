use std::{fmt, str::FromStr};

use csv::ByteRecord;

use crate::err::GtfsErr;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StopId {
    pub station: u16,
    pub child: Option<u8>,
}

impl StopId {
    #[inline]
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, GtfsErr> {
        const INVALID: GtfsErr = GtfsErr::GtfsFormatErr("Invalid stop id");
        let digits = |b: &[u8]| {
            if b.is_empty() || b.len() > 5 || !b.iter().all(u8::is_ascii_digit) {
                return Err(INVALID);
            }
            Ok(crate::num::unchecked_parse_u32(b))
        };
        let bytes = bytes.trim_ascii();
        let (station, child) = match bytes.iter().position(|&b| b == b'.') {
            Some(dot) => (&bytes[..dot], Some(&bytes[dot + 1..])),
            None => (bytes, None),
        };
        let child = match child {
            Some(child) => Some(u8::try_from(digits(child)?).map_err(|_| INVALID)?),
            None => None,
        };
        Ok(Self {
            station: u16::try_from(digits(station)?).map_err(|_| INVALID)?,
            child,
        })
    }
}

impl FromStr for StopId {
    type Err = GtfsErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s.as_bytes())
    }
}

impl fmt::Display for StopId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.child {
            Some(child) => write!(f, "{}.{}", self.station, child),
            None => write!(f, "{}", self.station),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LocationType {
    #[default]
    Platform,
    Station,
    Entrance,
}

#[derive(Default, Debug)]
/// a record like 41,,Santurtzi,  43.32895,  -3.03421,B2,1,
pub struct Stop {
    pub id: StopId,
    pub code: String,
    pub name: String,
    pub lat: f32,
    pub lon: f32,
    // TODO: figure out all zone ids and parse them without allocs
    pub zone_id: String,
    pub location_type: LocationType,
    pub parent_station: Option<StopId>,
}

pub struct Stops<R> {
    pub(crate) reader: csv::Reader<R>,
    pub(crate) reused_record: ByteRecord,
}

impl<R: std::io::Read> Stops<R> {
    fn field<T>(&self, index: usize) -> Result<T, GtfsErr>
    where
        T: FromStr,
    {
        self.field_str(index)?
            .trim()
            .parse()
            .map_err(|_| GtfsErr::GtfsFormatErr("Failed to parse field"))
    }

    fn field_str(&self, index: usize) -> Result<&str, GtfsErr> {
        let raw = self
            .reused_record
            .get(index)
            .ok_or(GtfsErr::GtfsFormatErr("Missing field"))?;

        str::from_utf8(raw).map_err(|_| GtfsErr::GtfsFormatErr("Invalid UTF-8"))
    }

    fn try_next(&mut self) -> Result<Option<Stop>, GtfsErr> {
        let has_record = self.reader.read_byte_record(&mut self.reused_record)?;

        // EOF
        if !has_record {
            return Ok(None);
        }

        let location_type = match self.field_str(6)?.trim() {
            "" | "0" => LocationType::Platform,
            "1" => LocationType::Station,
            "2" => LocationType::Entrance,
            _ => return Err(GtfsErr::GtfsFormatErr("Unsupported stop location_type")),
        };
        let parent_station = match self.field_str(7)?.trim() {
            "" => None,
            id => Some(id.parse()?),
        };

        Ok(Some(Stop {
            id: self.field(0)?,
            code: self.field_str(1)?.trim().to_owned(),
            name: self.field_str(2)?.to_owned(),
            lat: self.field(3)?,
            lon: self.field(4)?,
            zone_id: self.field_str(5)?.to_owned(),
            location_type,
            parent_station,
        }))
    }
}

impl<R: std::io::Read> Iterator for Stops<R> {
    type Item = Result<Stop, GtfsErr>;

    // stop_id,stop_code,stop_name,stop_lat,stop_lon,zone_id,location_type,parent_station
    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &str = "\
stop_id,stop_code,stop_name,stop_lat,stop_lon,zone_id,location_type,parent_station
41,,Santurtzi,  43.32895,  -3.03421,B2,1,
6,,Zazpikaleak/Casco Viejo,  43.25968,  -2.92071,A,1,
16,,Astrabudua,  43.31463,  -2.98681,B1,1,
25,,Ibarbengoa,  43.36320,  -3.00699,B1,1,
32,,Gurutzeta/Cruces,  43.28301,  -2.98245,B2,1,
34,,Barakaldo,  43.29563,  -2.98936,B2,1,
1,,Ariz,  43.23801,  -2.88203,B0,1,
15,,Erandio,  43.30338,  -2.97437,B1,1,
";

    fn stops(input: &str) -> Stops<std::io::Cursor<&[u8]>> {
        Stops {
            reader: csv::Reader::from_reader(std::io::Cursor::new(input.as_bytes())),
            reused_record: ByteRecord::new(),
        }
    }

    fn station(station: u16) -> StopId {
        StopId {
            station,
            child: None,
        }
    }

    #[test]
    fn parses_stops() {
        let stops = stops(INPUT);

        let stops: Vec<_> = stops.map(Result::unwrap).collect();

        assert_eq!(stops.len(), 8);

        assert_eq!(stops[0].id, station(41));
        assert_eq!(stops[0].name, "Santurtzi");
        assert!((stops[0].lat - 43.32895).abs() < 1e-5);
        assert!((stops[0].lon - -3.03421).abs() < 1e-5);
        assert_eq!(stops[0].zone_id, "B2");
        assert_eq!(stops[0].location_type, LocationType::Station);
        assert_eq!(stops[0].parent_station, None);

        assert_eq!(stops[1].id, station(6));
        assert_eq!(stops[1].name, "Zazpikaleak/Casco Viejo");
        assert_eq!(stops[1].zone_id, "A");

        assert_eq!(stops[7].id, station(15));
        assert_eq!(stops[7].name, "Erandio");
        assert_eq!(stops[7].zone_id, "B1");
    }

    #[test]
    fn parses_platforms_and_entrances() {
        let input = "\
stop_id,stop_code,stop_name,stop_lat,stop_lon,zone_id,location_type,parent_station
40.0,PEN,Peñota,  43.32198,  -3.02540,B2,0,40
1.1,,40 (Ariz),  43.23767,  -2.88255,B0,2,1
";
        let stops: Vec<_> = stops(input).map(Result::unwrap).collect();

        assert_eq!(
            stops[0].id,
            StopId {
                station: 40,
                child: Some(0)
            }
        );
        assert_eq!(stops[0].code, "PEN");
        assert_eq!(stops[0].location_type, LocationType::Platform);
        assert_eq!(stops[0].parent_station, Some(station(40)));

        assert_eq!(
            stops[1].id,
            StopId {
                station: 1,
                child: Some(1)
            }
        );
        assert_eq!(stops[1].location_type, LocationType::Entrance);
        assert_eq!(stops[1].parent_station, Some(station(1)));
    }

    #[test]
    fn stop_id_roundtrips_through_display() {
        for id in ["40", "40.0", "51.3"] {
            assert_eq!(id.parse::<StopId>().unwrap().to_string(), id);
        }
        assert!("".parse::<StopId>().is_err());
        assert!("4a".parse::<StopId>().is_err());
        assert!("70000".parse::<StopId>().is_err());
    }

    #[test]
    fn stops_to_result_propagates_parse_errors() {
        let input = "\
stop_id,stop_code,stop_name,stop_lat,stop_lon,zone_id,location_type,parent_station
41,,Santurtzi,43.32895,-3.03421,B2,1,
not-an-id,,Broken,43.3,-3.0,B1,1,
15,,Erandio,43.30338,-2.97437,B1,1,
";

        let result: Result<Vec<_>, _> = stops(input).collect();

        assert!(result.is_err());

        // The invalid row terminates iteration, so the third row
        // is never returned.
        let error = result.unwrap_err();

        dbg!(&error);

        assert!(matches!(error, GtfsErr::GtfsFormatErr(_)));
    }

    #[test]
    fn missing_field_is_an_error() {
        let input = "\
stop_id,stop_code,stop_name,stop_lat,stop_lon,zone_id,location_type,parent_station
41,,Santurtzi,43.32895,-3.03421
";

        let mut stops = stops(input);

        assert!(stops.next().unwrap().is_err());
        assert!(stops.next().is_none());
    }
}
