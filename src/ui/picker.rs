//! The picker window shown when a link is clicked (egui / eframe).

use crate::browsers::{self, Browser};

struct PickerApp {
    url: String,
    browsers: Vec<Browser>,
}

pub fn show(url: String) {
    let me = std::env::current_exe().ok();
    let browsers = browsers::detect(me.as_deref());
    if browsers.is_empty() {
        crate::msg("No browsers were detected on this system.");
        return;
    }

    let rows = browsers.len().min(12) as f32;
    let width = 380.0_f32;
    let height = 104.0 + rows * 50.0 + 30.0;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width, height])
            .with_resizable(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_title(crate::PRODUCT_NAME),
        centered: true,
        ..Default::default()
    };

    let app = PickerApp { url, browsers };
    let _ = eframe::run_native(
        crate::PRODUCT_NAME,
        options,
        Box::new(|cc| {
            crate::ui::apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    );
}

impl eframe::App for PickerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.106, 0.106, 0.106, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut launch_idx: Option<usize> = None;
        let mut cancel = false;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                cancel = true;
            }
            const NUM: [egui::Key; 9] = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
            ];
            for (n, k) in NUM.iter().enumerate() {
                if n < self.browsers.len() && i.key_pressed(*k) {
                    launch_idx = Some(n);
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading("Open link in…");
            ui.add_space(2.0);
            ui.colored_label(egui::Color32::from_gray(150), truncate(&self.url, 56));
            ui.add_space(12.0);

            for (n, b) in self.browsers.iter().enumerate() {
                let text = if n < 9 {
                    format!("  {}      {}", n + 1, b.name)
                } else {
                    format!("         {}", b.name)
                };
                let resp = ui.add_sized([ui.available_width(), 44.0], egui::Button::new(text));
                if resp.clicked() {
                    launch_idx = Some(n);
                }
                ui.add_space(6.0);
            }

            ui.add_space(2.0);
            ui.colored_label(
                egui::Color32::from_gray(120),
                "1–9 to pick   ·   Esc to cancel",
            );
        });

        if cancel {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if let Some(n) = launch_idx {
            let _ = browsers::launch(&self.browsers[n], &self.url);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}
