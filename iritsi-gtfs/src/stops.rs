pub struct Stop<'g> {
    pub stop_id: i16,
    pub stop_name: &'g str,
    pub stop_lat: &'g str,
    pub stop_lon: &'g str,
    pub zone_id: &'g str,
    pub location_type: i8,
}
