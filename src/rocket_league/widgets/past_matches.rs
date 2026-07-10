use super::{super::matches::MatchesServiceState, match_renderer::MatchRenderer};
use eframe::egui;
use std::{cell::RefCell, rc::Rc};

pub struct PastMatchesWidget {
    state: Rc<RefCell<MatchesServiceState>>,
}

impl PastMatchesWidget {
    pub fn new(state: Rc<RefCell<MatchesServiceState>>) -> Self {
        PastMatchesWidget { state }
    }
}

impl egui::Widget for &PastMatchesWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.vertical(|ui| {
                ui.add_space(4.0);
                for prev_match in self.state.borrow().prev_matches.iter().rev() {
                    ui.add(egui::Separator::default().spacing(8.0));
                    ui.add(MatchRenderer::new(prev_match));
                }
            })
            .response
        })
        .response
    }
}
