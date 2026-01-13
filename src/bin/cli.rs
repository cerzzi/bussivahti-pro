// TÄMÄ ALKU MUUTTUU:
use bussivahti_pro::models::StopData;
use bussivahti_pro::{network, settings, ui}; // Tuodaan kirjastosta
// (Poista vanhat "mod models;" rivit jos niitä oli tässä tiedostossa)

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{collections::HashMap, io, sync::Arc, time::Duration};
use tokio::sync::RwLock;

type AppState = Arc<RwLock<HashMap<String, StopData>>>;

#[tokio::main]
async fn main() -> Result<()> {
    let settings = settings::Settings::new().expect("Virhe: Settings.toml puuttuu tai on viallinen!");
    let stop_order: Vec<String> = settings.stops.keys().cloned().collect(); 

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_state: AppState = Arc::new(RwLock::new(HashMap::new()));

    let state_clone = app_state.clone();
    let settings_clone = settings.clone();
    tokio::spawn(async move {
        loop {
            let new_data = network::fetch_all_stops(&settings_clone).await;
            {
                let mut w = state_clone.write().await;
                *w = new_data;
            }
            tokio::time::sleep(Duration::from_secs(settings_clone.update_interval)).await;
        }
    });

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = std::time::Instant::now();

    loop {
        {
            let data = app_state.read().await;
            terminal.draw(|f| ui::render(f, &data, &stop_order))?;
        }

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}