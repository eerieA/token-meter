use anyhow::Result;
use eframe::egui;
use rust_decimal::Decimal;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

mod aggregator;
mod domain;
mod providers;
mod storage;

use aggregator::UsageAggregator;
use storage::{is_cache_outdated, load_api_key, load_cache, save_api_key};

#[derive(Debug)]
enum BgMessage {
    FetchMonthToDate { api_key: String },
}

#[derive(Debug)]
enum UiMessage {
    Started,
    Success(Decimal),
    Failed(String),
}

enum AppState {
    Setup,
    Main,
}

struct TokenMeterApp {
    state: AppState,
    api_key: String,
    status: String,
    total: Option<Decimal>,
    ui_rx: Receiver<UiMessage>,
    bg_tx: Sender<BgMessage>,
    setup_input: String,
    drag_icon_texture: Option<egui::TextureHandle>,
}

impl Default for TokenMeterApp {
    fn default() -> Self {
        let (ui_tx, ui_rx) = mpsc::channel::<UiMessage>();
        let (bg_tx, bg_rx) = mpsc::channel::<BgMessage>();

        // Spawn background worker thread which runs a tokio runtime and processes requests
        thread::spawn(move || {
            // create runtime
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            // background loop
            for req in bg_rx.iter() {
                let ui_tx = ui_tx.clone();
                match req {
                    BgMessage::FetchMonthToDate { api_key } => {
                        // spawn an async task to fetch and then send back result
                        rt.spawn(async move {
                            ui_tx.send(UiMessage::Started).ok();
                            let agg = UsageAggregator::new(&api_key);
                            match agg.fetch_month_to_date().await {
                                Ok(total) => {
                                    ui_tx.send(UiMessage::Success(total)).ok();
                                }
                                Err(e) => {
                                    ui_tx.send(UiMessage::Failed(format!("{}", e))).ok();
                                }
                            }
                        });
                    }
                }
            }
        });

        // Check if we have a saved API key
        if let Some(saved_key) = load_api_key() {
            // We have an API key, go to main state
            let mut app = Self {
                state: AppState::Main,
                api_key: saved_key.clone(),
                status: "Loading...".to_string(),
                total: None,
                ui_rx,
                bg_tx,
                setup_input: String::new(),
                drag_icon_texture: None,
            };

            // Try to load cached data or fetch if needed
            if let Some(cache) = load_cache() {
                if !is_cache_outdated() {
                    // Use cached data
                    if let Some(total_str) = cache.openai_total {
                        app.total = total_str.parse().ok();
                        app.status = "Loaded from cache".to_string();
                    }
                } else {
                    // Cache is outdated, fetch new data
                    app.bg_tx
                        .send(BgMessage::FetchMonthToDate { api_key: saved_key })
                        .ok();
                }
            } else {
                // No cache, fetch new data
                app.bg_tx
                    .send(BgMessage::FetchMonthToDate { api_key: saved_key })
                    .ok();
            }

            app
        } else {
            // No API key, show setup window
            Self {
                state: AppState::Setup,
                api_key: String::new(),
                status: "Setup required".to_string(),
                total: None,
                ui_rx,
                bg_tx,
                setup_input: String::new(),
                drag_icon_texture: None,
            }
        }
    }
}

impl eframe::App for TokenMeterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load drag icon texture if not already loaded
        if self.drag_icon_texture.is_none() {
            if let Some(image_data) = self.load_drag_icon() {
                self.drag_icon_texture = Some(ctx.load_texture(
                    "drag_handle",
                    image_data,
                    egui::TextureOptions::default(),
                ));
            }
        }

        // process any UI messages from background
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMessage::Started => {
                    self.status = "Fetching...".into();
                }
                UiMessage::Success(total) => {
                    self.total = Some(total);
                    self.status = "Fetched".into();
                    // Save to cache
                    if let Err(_) = storage::save_cache(&total) {
                        // Log error but don't fail
                    }
                }
                UiMessage::Failed(err) => {
                    self.status = format!("Failed: {}", err);
                }
            }
            ctx.request_repaint();
        }

        match self.state {
            AppState::Setup => {
                self.show_setup_window(ctx);
            }
            AppState::Main => {
                self.show_main_widget(ctx);
            }
        }
    }
}

impl TokenMeterApp {
    fn load_drag_icon(&self) -> Option<egui::ColorImage> {
        let image_data = include_bytes!("../assets/icons/drag_handle.png");
        let image = image::load_from_memory(image_data).ok()?;
        let rgba = image.to_rgba8();
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            &rgba,
        ))
    }

    fn show_setup_window(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Token Meter Setup");
            ui.add_space(20.0);

            ui.label("Please enter your OpenAI API key:");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("API Key:");
                ui.text_edit_singleline(&mut self.setup_input);
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if ui.button("Save & Start").clicked() {
                    if !self.setup_input.is_empty() {
                        if let Err(e) = save_api_key(&self.setup_input) {
                            self.status = format!("Failed to save: {}", e);
                        } else {
                            // Switch to main state and start fetching
                            self.state = AppState::Main;
                            self.api_key = self.setup_input.clone();
                            self.status = "Fetching...".to_string();
                            self.bg_tx
                                .send(BgMessage::FetchMonthToDate {
                                    api_key: self.setup_input.clone(),
                                })
                                .ok();
                        }
                    }
                }

                if ui.button("Cancel").clicked() {
                    ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
                }
            });

            if !self.status.is_empty() && self.status != "Setup required" {
                ui.add_space(10.0);
                ui.label(&self.status);
            }
        });
    }

    fn show_main_widget(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Top row with drag icon on the right
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() - 16.0); // Push icon to the right

                let drag_response = if let Some(texture) = &self.drag_icon_texture {
                    ui.add(
                        egui::Image::from_texture(texture)
                            .fit_to_exact_size(egui::vec2(16.0, 16.0))
                            .sense(egui::Sense::click_and_drag()),
                    )
                } else {
                    // Fallback to text if image not loaded
                    ui.add_sized(
                        [16.0, 16.0],
                        egui::Label::new("⋮⋮").sense(egui::Sense::click_and_drag()),
                    )
                };

                if drag_response.drag_started() {
                    ctx.send_viewport_cmd(egui::viewport::ViewportCommand::StartDrag);
                }
            });

            ui.add_space(4.0);

            // the cost text
            if let Some(total) = &self.total {
                ui.label(
                    egui::RichText::new(format!("MTD: ${:.2}", total))
                        .size(40.0)
                        .color(egui::Color32::from_rgb(0, 255, 0)),
                );
            } else {
                ui.label(egui::RichText::new("MTD: --").size(40.0));
            }
            // the status text
            ui.label(format!("Status: {}", self.status));

            ui.horizontal(|ui| {
                if ui.button("📊 Refresh").clicked() {
                    let api = self.api_key.clone();
                    self.bg_tx
                        .send(BgMessage::FetchMonthToDate { api_key: api })
                        .ok();
                }
            });
        });
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([224.0, 150.0])
            .with_min_inner_size([184.0, 120.0])
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "token-meter-egui",
        native_options,
        Box::new(|_cc| Box::new(TokenMeterApp::default())),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))?;

    Ok(())
}
