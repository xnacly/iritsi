use crate::{err::GtfsErr, num};

/// trip_id,stop_sequence,arrival_time,departure_time,stop_id,pickup_type
/// 870583,3,09:53:02,09:53:20,40.0,0
/// 870583,4,09:54:22,09:54:37,39.0,0
/// 870583,5,09:55:42,09:55:59,38.0,0
#[derive(Debug, PartialEq)]
pub struct StopTime {
    trip_id: u32,
    stop_seq: u8,
    arrival_time: u32,
    departure_time: u32,
    stop_id: f32,
}

impl TryFrom<&[u8]> for StopTime {
    type Error = GtfsErr;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut fields = value.split(|&b| b == b',');

        let trip_id = num::unchecked_parse_u32(fields.next().unwrap());
        let stop_seq = num::unchecked_parse_u32(fields.next().unwrap()) as u8;
        let arrival_time = num::unchecked_parse_time(fields.next().unwrap());
        let departure_time = num::unchecked_parse_time(fields.next().unwrap());

        // PERF: technically this is not a floating point but rather <stop_id>.0; i dont know why
        let stop_id = unsafe { str::from_utf8_unchecked(fields.next().unwrap()) }
            .parse::<f32>()
            .map_err(|_| GtfsErr::GtfsFormatErr("Failed to parse stop_times.stop_id as float"))?;

        Ok(Self {
            trip_id,
            stop_seq,
            arrival_time,
            departure_time,
            stop_id,
        })
    }
}

/// Stop times is at least 170k rows, 6 numeric fields each, since the csv package does a lot of
/// copying, I decided to use a hand rolled csv parser for this to only use slices into the input
/// without copying anything since this input has no quotes or other csv weirdnesses
pub struct StopTimes {
    src: Vec<u8>,
    pos: usize,
}

impl From<Vec<u8>> for StopTimes {
    fn from(value: Vec<u8>) -> Self {
        let pos = if value.starts_with(b"trip_id,") {
            value
                .iter()
                .position(|&b| b == b'\n')
                .map_or(value.len(), |header_end| header_end + 1)
        } else {
            0
        };
        Self { src: value, pos }
    }
}

impl Iterator for StopTimes {
    type Item = Result<StopTime, GtfsErr>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.src.len() {
            return None;
        }

        let rest = &self.src[self.pos..];
        let end = rest
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or_else(|| rest.len());

        let line = &rest[..end];
        self.pos += end + (end < rest.len()) as usize;
        Some(line.try_into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop_time(
        trip_id: u32,
        stop_seq: u8,
        arrival_time: u32,
        departure_time: u32,
        stop_id: f32,
    ) -> StopTime {
        StopTime {
            trip_id,
            stop_seq,
            arrival_time,
            departure_time,
            stop_id,
        }
    }

    #[test]
    fn parses_stop_time() {
        let input = b"870583,3,09:53:02,09:53:20,40.0,0";

        let actual = StopTime::try_from(input.as_slice()).unwrap();

        assert_eq!(
            actual,
            stop_time(
                870583,
                3,
                9 * 3600 + 53 * 60 + 2,
                9 * 3600 + 53 * 60 + 20,
                40.0,
            )
        );
    }

    #[test]
    fn parses_multiple_stop_times() {
        let input = b"\
870583,3,09:53:02,09:53:20,40.0,0
870583,4,09:54:22,09:54:37,39.0,0
870583,5,09:55:42,09:55:59,38.0,0
";

        let stop_times: Vec<_> = StopTimes::from(input.to_vec())
            .map(Result::unwrap)
            .collect();

        assert_eq!(
            stop_times,
            vec![
                stop_time(
                    870583,
                    3,
                    9 * 3600 + 53 * 60 + 2,
                    9 * 3600 + 53 * 60 + 20,
                    40.0,
                ),
                stop_time(
                    870583,
                    4,
                    9 * 3600 + 54 * 60 + 22,
                    9 * 3600 + 54 * 60 + 37,
                    39.0,
                ),
                stop_time(
                    870583,
                    5,
                    9 * 3600 + 55 * 60 + 42,
                    9 * 3600 + 55 * 60 + 59,
                    38.0,
                ),
            ]
        );
    }

    #[test]
    fn parses_final_line_without_newline() {
        let input = b"870583,3,09:53:02,09:53:20,40.0,0";

        let mut stop_times = StopTimes::from(input.to_vec());

        let actual = stop_times.next().unwrap().unwrap();

        assert_eq!(
            actual,
            stop_time(
                870583,
                3,
                9 * 3600 + 53 * 60 + 2,
                9 * 3600 + 53 * 60 + 20,
                40.0,
            )
        );

        assert!(stop_times.next().is_none());
    }

    #[test]
    fn handles_trailing_newline() {
        let input = b"870583,3,09:53:02,09:53:20,40.0,0\n";

        let mut stop_times = StopTimes::from(input.to_vec());

        assert!(stop_times.next().unwrap().is_ok());
        assert!(stop_times.next().is_none());
    }

    #[test]
    fn empty_input_produces_no_records() {
        let mut stop_times = StopTimes::from(Vec::new());

        assert!(stop_times.next().is_none());
    }

    #[test]
    fn skips_the_header_row() {
        let input = b"trip_id,stop_sequence,arrival_time,departure_time,stop_id,pickup_type\n870583,3,09:53:02,09:53:20,40.0,0\n";

        let actual = StopTimes::from(input.to_vec()).next().unwrap().unwrap();

        assert_eq!(actual.trip_id, 870583);
    }

    #[test]
    fn supports_variable_length_trip_ids() {
        let input = b"123456789,3,09:53:02,09:53:20,40.0,0";

        let actual = StopTime::try_from(input.as_slice()).unwrap();

        assert_eq!(actual.trip_id, 123456789);
    }

    #[test]
    fn supports_maximum_u32_trip_id() {
        let input = b"4294967295,3,09:53:02,09:53:20,40.0,0";

        let actual = StopTime::try_from(input.as_slice()).unwrap();

        assert_eq!(actual.trip_id, u32::MAX);
    }

    #[test]
    fn supports_gtfs_times_after_midnight() {
        let input = b"870583,3,25:30:02,26:00:20,40.0,0";

        let actual = StopTime::try_from(input.as_slice()).unwrap();

        assert_eq!(actual.arrival_time, 25 * 3600 + 30 * 60 + 2);
        assert_eq!(actual.departure_time, 26 * 3600 + 20);
    }

    #[test]
    fn parses_midnight() {
        let input = b"870583,1,00:00:00,00:00:00,40.0,0";

        let actual = StopTime::try_from(input.as_slice()).unwrap();

        assert_eq!(actual.arrival_time, 0);
        assert_eq!(actual.departure_time, 0);
    }

    #[test]
    fn parses_stop_id_as_float() {
        let input = b"870583,3,09:53:02,09:53:20,40.5,0";

        let actual = StopTime::try_from(input.as_slice()).unwrap();

        assert_eq!(actual.stop_id, 40.5);
    }
}
