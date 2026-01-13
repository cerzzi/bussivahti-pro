use serde::Deserialize;

// --- DIGITRANSIT ROUTING API (Aikataulut) ---

#[derive(Deserialize, Debug, Clone)]
pub struct GqlResponse {
    pub data: DataData,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DataData {
    pub stop: Option<Stop>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Stop {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(rename = "stoptimesWithoutPatterns")]
    pub stoptimes: Vec<StopTime>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StopTime {
    #[serde(rename = "realtimeDeparture")]
    pub realtime_departure: i64,
    #[serde(rename = "scheduledDeparture")]
    pub scheduled_departure: i64,
    pub realtime: bool,
    pub trip: Trip,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Trip {
    pub route: Route,
    #[serde(rename = "tripHeadsign")]
    pub headsign: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Route {
    #[serde(rename = "shortName")]
    pub short_name: String,
}

// --- SISÄINEN TIETORAKENNE (UI) ---

#[derive(Debug, Clone)]
pub struct DepartureInfo {
    pub line: String,
    pub headsign: String,
    pub time_str: String,
    pub minutes_left: i64,
    pub seconds_left: i64,
    pub is_realtime: bool,
}

#[derive(Debug, Clone)]
pub struct StopData {
    pub stop_id: String,
    pub stop_name: String,
    pub lat: f64,
    pub lon: f64,
    pub departures: Vec<DepartureInfo>,
    pub last_updated: chrono::DateTime<chrono::Local>,
}

// --- GEOCODING API (Haku) - KORJATTU ---

#[derive(Deserialize, Debug, Clone)]
pub struct GeoResponse {
    pub features: Vec<GeoFeature>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GeoFeature {
    pub properties: GeoProperties,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GeoProperties {
    pub name: String,
    pub label: String,
    
    // KORJAUS 1: JSONissa kenttä on "id", ei "gtfsId"
    #[serde(rename = "id")]
    pub gtfs_id: Option<String>, 
    
    // KORJAUS 2: Koodi on täällä sisällä
    pub addendum: Option<GeoAddendum>,
}

// Uudet apurakenteet koodin kaivamiseen
#[derive(Deserialize, Debug, Clone)]
pub struct GeoAddendum {
    #[serde(rename = "GTFS")]
    pub gtfs: Option<GeoGtfs>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GeoGtfs {
    pub code: Option<String>,
}