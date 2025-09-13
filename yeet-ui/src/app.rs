use std::sync::Arc;

use egui::mutex::RwLock;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[derive(serde::Deserialize, serde::Serialize)]
// #[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Yeet {
    yeet_status: Arc<RwLock<String>>,
}

impl Default for Yeet {
    fn default() -> Self {
        Self {
            yeet_status: Arc::new(RwLock::new("Nothing here yet".to_owned())),
        }
    }
}

impl Yeet {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // TODO
        // if let Some(storage) = cc.storage {
        //     eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        // } else {
        Default::default()
        // }
    }
}

impl eframe::App for Yeet {
    /// Called by the framework to save state before shutdown.
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Yeet.rs");

            if ui.button("Fetch status").clicked() {
                let status = Arc::clone(&self.yeet_status);
                let request = ehttp::Request::get("api/status");
                ehttp::fetch(request, move |response| {
                    *status.write() = match response {
                        Ok(response) => response.text().unwrap_or("Empty response").to_owned(),
                        Err(err) => err,
                    };
                });
            }

            ui.label(&*self.yeet_status.read());

            ui.separator();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
