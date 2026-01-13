use crate::models::*;
use crate::settings::Settings;
use anyhow::Result;
use chrono::{Local, TimeZone};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

const API_URL: &str = "https://api.digitransit.fi/routing/v2/waltti/gtfs/v1";

#[derive(Serialize)]
struct GqlQuery {
    query: String,
}

pub async fn fetch_all_stops(settings: &Settings) -> HashMap<String, StopData> {
    let client = Client::new();
    let mut tasks = vec![];
    
    let stops_config = settings.stops.clone();
    let api_key = Arc::new(settings.api_key.clone());

    for (stop_id, wanted_lines) in stops_config {
        let client = client.clone();
        let key = api_key.clone();
        let id_clone = stop_id.clone();

        tasks.push(tokio::spawn(async move {
            let data = fetch_stop(&client, &key, &id_clone, &wanted_lines).await;
            (id_clone, data)
        }));
    }

    let mut results = HashMap::new();
    for task in futures::future::join_all(tasks).await {
        if let Ok((id, Ok(stop_data))) = task {
            results.insert(id, stop_data);
        }
    }
    results
}

async fn fetch_stop(client: &Client, api_key: &str, stop_id: &str, wanted_lines: &[String]) -> Result<StopData> {
    let query = format!(
        r#"{{
          stop(id: "{}") {{
            name
            lat
            lon
            stoptimesWithoutPatterns(numberOfDepartures: 20) {{
              realtimeDeparture
              scheduledDeparture
              realtime
              trip {{ route {{ shortName }} tripHeadsign }}
            }}
          }}
        }}"#,
        stop_id
    );

    let resp = client.post(API_URL)
        .header("digitransit-subscription-key", api_key)
        .header("Content-Type", "application/json")
        .json(&GqlQuery { query })
        .send().await?
        .json::<GqlResponse>().await?;

    let stop = resp.data.stop.ok_or(anyhow::anyhow!("Stop not found"))?;
    let now = Local::now();
    let today_midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap();

    let mut departures = Vec::new();

    for st in stop.stoptimes {
        let line = st.trip.route.short_name.clone();
        
        // "ALL"-tuki: Hyväksy jos listalla "ALL" tai kyseinen linja
        let accept_all = wanted_lines.contains(&"ALL".to_string());
        if !accept_all && !wanted_lines.contains(&line) { 
            continue; 
        }

        let seconds_from_midnight = if st.realtime { st.realtime_departure } else { st.scheduled_departure };
        let duration = chrono::Duration::seconds(seconds_from_midnight as i64);
        let dep_naive = today_midnight + duration;
        let final_time = Local.from_local_datetime(&dep_naive).single().unwrap_or(now);
        let seconds_left = final_time.signed_duration_since(now).num_seconds();
        if seconds_left < 0 { continue; }

        departures.push(DepartureInfo {
            line,
            headsign: st.trip.headsign,
            time_str: final_time.format("%H:%M").to_string(),
            minutes_left: seconds_left / 60,
            seconds_left,
            is_realtime: st.realtime,
        });
    }

    departures.sort_by_key(|d| d.seconds_left);
    departures.truncate(5);

    Ok(StopData {
        stop_id: stop_id.to_string(),
        stop_name: stop.name,
        lat: stop.lat,
        lon: stop.lon,
        departures,
        last_updated: Local::now(),
    })
}

// Haku (Geocoding API)
// src/network.rs loppuun:

// Muutos: lisätty api_key parametriksi
// src/network.rs

// src/network.rs

pub async fn search_stops(text: &str, api_key: &str) -> Result<Vec<GeoProperties>> {
    let client = Client::new();
    let url = "https://api.digitransit.fi/geocoding/v1/search";
    
    println!("DEBUG: Haetaan hakusanalla: '{}'", text);

    let resp = client.get(url)
        .query(&[
            ("text", text),
            ("size", "10"),
            // ("sources", "gtfs"), // Tämä on poistettu, jotta löytyy kaikki
            ("layers", "stop"),     // Rajataan pysäkkeihin
            
            // Tampereen aluerajaus
            ("boundary.rect.min_lat", "61.4"),
            ("boundary.rect.max_lat", "61.6"),
            ("boundary.rect.min_lon", "23.5"),
            ("boundary.rect.max_lon", "24.0"),
        ])
        .header("digitransit-subscription-key", api_key)
        .send()
        .await?;

    println!("DEBUG: API vastaus status: {}", resp.status());

    if !resp.status().is_success() {
        println!("DEBUG: Virhe haussa! Status koodi: {}", resp.status());
        if let Ok(err_text) = resp.text().await {
             println!("DEBUG: Virheviesti: {}", err_text);
        }
        return Err(anyhow::anyhow!("API request failed"));
    }

    // --- DEBUGGAUS ALKAA ---
    // 1. Luetaan vastaus tekstinä (ei suoraan JSONina), jotta voimme tulostaa sen
    let raw_text = resp.text().await?;
    
    // 2. Tulostetaan terminaaliin. TÄMÄ ON SE TÄRKEIN RIVI NYT!
    // Etsi tulosteesta kohta, jossa on pysäkin ID (esim. "gtfsId": "..." tai "id": "...")
    println!("DEBUG RAW JSON: {}", raw_text); 

    // 3. Parsitaan teksti JSON-olioksi
    let json: GeoResponse = serde_json::from_str(&raw_text)?;
    // --- DEBUGGAUS PÄÄTTYY ---

    println!("DEBUG: Löydetty {} tulosta", json.features.len());

    let results = json.features.into_iter().map(|f| f.properties).collect();
    Ok(results)
}