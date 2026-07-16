use std::{cell::RefCell, rc::Rc};

use eframe::egui;

pub struct AppSettingsWidget {
    transparency: Rc<RefCell<u8>>,
}

impl AppSettingsWidget {
    pub fn new(transparency: Rc<RefCell<u8>>) -> Self {
        AppSettingsWidget { transparency }
    }
}

impl egui::Widget for &AppSettingsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            ui.add(
                egui::Slider::new(&mut *self.transparency.borrow_mut(), u8::MIN..=u8::MAX)
                    .text("App transparency"),
            );
        })
        .response
    }
}
