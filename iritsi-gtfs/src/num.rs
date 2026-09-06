/// Optimised, nonvalidating u32 parsing
#[inline]
pub fn unchecked_parse_u32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0, |n, &b| n * 10 + (b - b'0') as u32)
}

#[inline]
pub fn unchecked_parse_time(s: &[u8]) -> u32 {
    let hour = (s[0] - b'0') as u32 * 10 + (s[1] - b'0') as u32;
    let minute = (s[3] - b'0') as u32 * 10 + (s[4] - b'0') as u32;
    let second = (s[6] - b'0') as u32 * 10 + (s[7] - b'0') as u32;
    hour * 3600 + minute * 60 + second
}
