use std::str::FromStr;

use csv::ByteRecord;

use crate::err::GtfsErr;

#[derive(Default, Debug)]
/// a record like 41,,Santurtzi,  43.32895,  -3.03421,B2,1,
pub struct Stop {
    pub id: i16,
    pub name: String,
    pub lat: f32,
    pub lon: f32,
    // TODO: figure out all zone ids and parse them without allocs
    pub zone_id: String,
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

        Ok(Some(Stop {
            id: self.field(0)?,
            name: self.field_str(2)?.to_owned(),
            lat: self.field(3)?,
            lon: self.field(4)?,
            zone_id: self.field_str(5)?.to_owned(),
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

    #[test]
    fn parses_stops() {
        let stops = stops(INPUT);

        let stops: Vec<_> = stops.map(Result::unwrap).collect();

        assert_eq!(stops.len(), 8);

        assert_eq!(stops[0].id, 41);
        assert_eq!(stops[0].name, "Santurtzi");
        assert!((stops[0].lat - 43.32895).abs() < 1e-5);
        assert!((stops[0].lon - -3.03421).abs() < 1e-5);
        assert_eq!(stops[0].zone_id, "B2");

        assert_eq!(stops[1].id, 6);
        assert_eq!(stops[1].name, "Zazpikaleak/Casco Viejo");
        assert_eq!(stops[1].zone_id, "A");

        assert_eq!(stops[7].id, 15);
        assert_eq!(stops[7].name, "Erandio");
        assert_eq!(stops[7].zone_id, "B1");
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
