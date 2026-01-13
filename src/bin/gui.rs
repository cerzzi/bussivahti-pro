use bussivahti_pro::{models::{StopData, GeoProperties}, network, settings};
use eframe::egui;
use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};
use tokio::runtime::Runtime;
use walkers::{Map, MapMemory, HttpTiles, Position, sources::OpenStreetMap, Plugin, Projector};

// --- GUI STRUCT ---
struct BussivahtiGui {
    tiles: HttpTiles,
    map_memory: MapMemory,
    stops: Arc<Mutex<HashMap<String, StopData>>>,
    rt: Runtime,
    settings: settings::Settings,

    // HAKU & NAVIGOINTI
    search_text: String,
    search_results: Arc<Mutex<Vec<GeoProperties>>>,
    is_searching: Arc<Mutex<bool>>,
    map_center_pos: Position,

    // UUSI: UI Skaalaus
    ui_scale: f32,
}

// --- PLUGIN STRUCT ---
struct BusMarkerPlugin<'a> {
    stops: &'a HashMap<String, StopData>,
}

impl<'a> Plugin for BusMarkerPlugin<'a> {
    fn run(self: Box<Self>, ui: &mut egui::Ui, _response: &egui::Response, projector: &Projector) {
        let painter = ui.painter();
        
        for stop in self.stops.values() {
            let position = Position::from_lon_lat(stop.lon, stop.lat);
            let screen_position = projector.project(position).to_pos2();

            // Määritä väri
            let min_minutes = stop.departures.first().map(|d| d.minutes_left).unwrap_or(99);
            let color = if min_minutes <= 2 { egui::Color32::RED }
                       else if min_minutes <= 5 { egui::Color32::YELLOW }
                       else { egui::Color32::GREEN };

            // 1. Piirrä pallo
            painter.circle_filled(screen_position, 10.0, color);
            
            // 2. Piirrä teksti
            let text = format!("{}\n{} min", stop.stop_name, min_minutes);
            let text_pos = screen_position + egui::vec2(0.0, 15.0);
            
            painter.text(
                text_pos, 
                egui::Align2::CENTER_TOP, 
                text, 
                egui::FontId::proportional(12.0), 
                egui::Color32::BLACK
            );

            // 3. TOOLTIP
            let hover_rect = egui::Rect::from_center_size(screen_position, egui::vec2(20.0, 20.0));
            let id = ui.id().with(&stop.stop_id); 
            
            let response = ui.interact(hover_rect, id, egui::Sense::hover());

            response.on_hover_ui(|ui| {
                ui.heading(&stop.stop_name);
                ui.separator();
                
                egui::Grid::new("departures_grid")
                    .striped(true)
                    .spacing([15.0, 4.0]) 
                    .show(ui, |ui| {
                        ui.strong("Linja");
                        ui.strong("Määränpää");
                        ui.strong("Aika");
                        ui.end_row();

                        for dep in &stop.departures {
                            let time_color = if dep.minutes_left <= 2 { egui::Color32::RED }
                                            else if dep.minutes_left <= 5 { egui::Color32::YELLOW }
                                            else { egui::Color32::GREEN };
                            
                            ui.strong(&dep.line);
                            ui.label(&dep.headsign);
                            ui.colored_label(time_color, format!("{} ({} min)", dep.time_str, dep.minutes_left));
                            ui.end_row();
                        }
                    });
                    
                if stop.departures.is_empty() {
                    ui.label("Ei lähtöjä lähiaikoina.");
                }
                
                ui.separator();
                ui.small(format!("Päivitetty: {}", stop.last_updated.format("%H:%M:%S")));
            });
        }
    }
}

impl BussivahtiGui {
    fn new(cc: &eframe::CreationContext<'_>, settings: settings::Settings) -> Self {
        let rt = Runtime::new().expect("Tokio runtime failed");
        let tiles = HttpTiles::new(OpenStreetMap, cc.egui_ctx.clone());

        let stops = Arc::new(Mutex::new(HashMap::new()));
        let stops_clone = stops.clone();
        let settings_clone = settings.clone();
        
        rt.spawn(async move {
            loop {
                let new_data = network::fetch_all_stops(&settings_clone).await;
                {
                    let mut lock = stops_clone.lock().unwrap();
                    for (k, v) in new_data {
                        lock.insert(k, v);
                    }
                }
                tokio::time::sleep(Duration::from_secs(settings_clone.update_interval)).await;
            }
        });

        Self {
            tiles,
            map_memory: MapMemory::default(),
            stops,
            rt,
            settings,
            search_text: String::new(),
            search_results: Arc::new(Mutex::new(Vec::new())),
            is_searching: Arc::new(Mutex::new(false)),
            map_center_pos: Position::from_lon_lat(23.76, 61.498),
            
            // Asetetaan oletusskaalaukseksi 1.3
            ui_scale: 1.3,
        }
    }

    fn trigger_search(&self) {
        let text = self.search_text.clone();
        let results_store = self.search_results.clone();
        let loading_flag = self.is_searching.clone();
        let api_key = self.settings.api_key.clone();
        
        self.rt.spawn(async move {
            *loading_flag.lock().unwrap() = true;
            match network::search_stops(&text, &api_key).await {
                Ok(results) => {
                    *results_store.lock().unwrap() = results;
                }
                Err(e) => {
                    println!("VIRHE HAUSSA: {:?}", e);
                }
            }
            *loading_flag.lock().unwrap() = false;
        });
    }

    fn add_stop_to_tracking(&self, stop_gtfs_id: String) {
        let stops_store = self.stops.clone();
        let mut temp_settings = self.settings.clone();
        
        let mut single_stop_map = HashMap::new();
        single_stop_map.insert(stop_gtfs_id.clone(), vec!["ALL".to_string()]);
        temp_settings.stops = single_stop_map;

        self.rt.spawn(async move {
            let new_data = network::fetch_all_stops(&temp_settings).await;
            if let Some(data) = new_data.get(&stop_gtfs_id) {
                let mut lock = stops_store.lock().unwrap();
                lock.insert(stop_gtfs_id, data.clone());
            }
        });
    }
}

impl eframe::App for BussivahtiGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        // KÄYTÄ SKAALAUSTA
        ctx.set_pixels_per_point(self.ui_scale);

        // --- SIVUPANEELI ---
        egui::SidePanel::left("menu_panel")
            .resizable(true)
            .default_width(350.0)
            .show(ctx, |ui| {
                
                // SKAALAUS SÄÄDIN
                ui.heading("Asetukset ⚙️");
                ui.horizontal(|ui| {
                    ui.label("Koko:");
                    ui.add(egui::Slider::new(&mut self.ui_scale, 0.8..=2.5).text("x"));
                });
                ui.separator();

                ui.heading("Haku 🔍");
                ui.separator();

                ui.horizontal(|ui| {
                    let response = ui.text_edit_singleline(&mut self.search_text);
                    if ui.button("Hae").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        self.trigger_search();
                    }
                });

                if *self.is_searching.lock().unwrap() {
                    ui.spinner();
                }

                ui.separator();

                // --- SCROLL ALUE ---
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        
                        // HAKUTULOKSET
                        let results = self.search_results.lock().unwrap().clone();
                        if !results.is_empty() {
                            ui.strong(format!("Tulokset ({}):", results.len()));
                            
                            for result in results {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        let code_str = result.addendum.as_ref()
                                            .and_then(|a| a.gtfs.as_ref())
                                            .and_then(|g| g.code.as_deref())
                                            .unwrap_or("-");

                                        ui.vertical(|ui| {
                                            ui.heading(format!("{} ({})", result.name, code_str));
                                            ui.label(&result.label);
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if let Some(raw_id) = &result.gtfs_id {
                                                if ui.button("➕").clicked() {
                                                    // SIIVOTAAN ID
                                                    let clean_id = raw_id
                                                        .replace("GTFS:", "")
                                                        .split('#').next()
                                                        .unwrap_or("")
                                                        .to_string();
                                                    self.add_stop_to_tracking(clean_id);
                                                }
                                            } else {
                                                ui.colored_label(egui::Color32::RED, "🚫 Ei ID");
                                            }
                                        });
                                    });
                                });
                            }
                            ui.separator();
                        }

                        // SEURATTAVAT
                        ui.heading("Seurannassa:");
                        let tracked = self.stops.lock().unwrap();
                        
                        for stop in tracked.values() {
                            if ui.button(format!("📍 {}", stop.stop_name)).clicked() {
                                self.map_center_pos = Position::from_lon_lat(stop.lon, stop.lat);
                                self.map_memory = MapMemory::default();
                            }
                        }
                    });
            });

        // --- KARTTAPANEELI ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let stops_data = {
                let lock = self.stops.lock().unwrap();
                lock.clone()
            };

            let map = Map::new(
                Some(&mut self.tiles),
                &mut self.map_memory,
                self.map_center_pos
            );

            let markers = BusMarkerPlugin { stops: &stops_data };
            ui.add(map.with_plugin(markers));
        });

        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

fn main() -> eframe::Result {
    let settings = bussivahti_pro::settings::Settings::new().expect("Config error");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Bussivahti Map",
        options,
        Box::new(|cc| Ok(Box::new(BussivahtiGui::new(cc, settings)))),
    )
}