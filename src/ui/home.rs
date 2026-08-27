//! Home / settings window shown when the app is opened directly (egui / eframe).

use crate::{browsers, register};

struct HomeApp {
    browsers: Vec<browsers::Browser>,
    status: String,
}

pub fn show() {
    let me = std::env::current_exe().ok();
    let browsers = browsers::detect(me.as_deref());
    let status = if register::is_registered() {
        "Registered. Set it as default in Settings ▸ Apps ▸ Default apps.".to_string()
    } else {
        "Not registered yet — click \u{201C}Set as default browser\u{201D}.".to_string()
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 470.0])
            .with_resizable(false)
            .with_title(crate::PRODUCT_NAME),
        centered: true,
        ..Default::default()
    };

    let app = HomeApp { browsers, status };
    if let Err(e) = eframe::run_native(
        crate::PRODUCT_NAME,
        options,
        Box::new(|cc| {
            crate::ui::apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ) {
        crate::msg(&format!(
            "Browser Picker couldn't open its window — the graphics renderer failed to start.\n\n{e}"
        ));
    }
}

impl eframe::App for HomeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.106, 0.106, 0.106, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let count = self.browsers.len();
        let items: Vec<String> = self
            .browsers
            .iter()
            .enumerate()
            .map(|(i, b)| format!("{}.   {}", i + 1, b.name))
            .collect();
        let status = self.status.clone();

        let mut do_set = false;
        let mut do_reg = false;
        let mut do_unreg = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading(crate::PRODUCT_NAME);
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::from_gray(160), status.as_str());
            ui.add_space(14.0);

            ui.label(format!("Detected browsers ({count}):"));
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(0x24, 0x24, 0x24))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if items.is_empty() {
                        ui.label("None detected.");
                    } else {
                        for it in &items {
                            ui.label(it.as_str());
                        }
                    }
                });

            ui.add_space(16.0);
            if ui
                .add_sized(
                    [ui.available_width(), 44.0],
                    egui::Button::new("Set as default browser"),
                )
                .clicked()
            {
                do_set = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let half = (ui.available_width() - 8.0) / 2.0;
                if ui
                    .add_sized([half, 36.0], egui::Button::new("Register"))
                    .clicked()
                {
                    do_reg = true;
                }
                if ui
                    .add_sized([half, 36.0], egui::Button::new("Unregister"))
                    .clicked()
                {
                    do_unreg = true;
                }
            });
        });

        if do_set {
            match register::register() {
                Ok(_) => {
                    open_default_apps();
                    self.status =
                        "Opened Default Apps — choose Browser Picker for HTTP and HTTPS.".into();
                }
                Err(e) => self.status = format!("Registration failed: {e}"),
            }
        } else if do_reg {
            self.status = match register::register() {
                Ok(_) => "Registered.".into(),
                Err(e) => format!("Failed: {e}"),
            };
        } else if do_unreg {
            let _ = register::unregister();
            self.status = "Unregistered.".into();
        }
    }
}

fn open_default_apps() {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps"])
        .spawn();
}
