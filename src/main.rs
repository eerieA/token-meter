use eframe::egui;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use anyhow::Result;
use rust_decimal::Decimal;

mod providers;
mod aggregator;
mod storage;
mod domain;

use aggregator::UsageAggregator;
use storage::{load_api_key, save_api_key};

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

struct TokenMeterApp {
    api_key: String,
    status: String,
    total: Option<Decimal>,
    ui_rx: Receiver<UiMessage>,
    bg_tx: Sender<BgMessage>,
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

        // attempt to load saved key
        let api_key = load_api_key().unwrap_or_default();

        Self {
            api_key,
            status: "Idle".to_string(),
            total: None,
            ui_rx,
            bg_tx,
        }
    }
}

impl eframe::App for TokenMeterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // process any UI messages from background
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMessage::Started => {
                    self.status = "Fetching...".into();
                }
                UiMessage::Success(total) => {
                    self.total = Some(total);
                    self.status = "Fetched".into();
                }
                UiMessage::Failed(err) => {
                    self.status = format!("Failed: {}", err);
                }
            }
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Add a draggable header area
            ui.horizontal(|ui| {
                let drag_response = ui.add_sized(
                    [ui.available_width(), 20.0],
                    egui::Label::new("📊 Token Meter").sense(egui::Sense::click_and_drag()),
                );
                if drag_response.drag_started() {
                    ctx.send_viewport_cmd(egui::viewport::ViewportCommand::StartDrag);
                }
            });
            
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("API key:");
                ui.text_edit_singleline(&mut self.api_key);
                if ui.button("💾").clicked() {
                    if !self.api_key.is_empty() {
                        if let Err(e) = save_api_key(&self.api_key) {
                            self.status = format!("Failed: {}", e);
                        } else {
                            self.status = "Saved".into();
                        }
                    }
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button("📊 Fetch").clicked() {
                    let api = self.api_key.clone();
                    self.bg_tx.send(BgMessage::FetchMonthToDate { api_key: api }).ok();
                }
                if ui.button("🗑️ Clear").clicked() {
                    self.total = None;
                    self.status = "Cleared".into();
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            ui.label(format!("Status: {}", self.status));
            if let Some(total) = &self.total {
                ui.label(format!("MTD: ${:.2}", total));
            } else {
                ui.label("MTD: --");
            }
        });
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 200.0])
            .with_min_inner_size([280.0, 150.0])
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };
    
    eframe::run_native(
        "token-meter-egui",
        native_options,
        Box::new(|_cc| Box::new(TokenMeterApp::default())),
    ).map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))?;

    Ok(())
}
