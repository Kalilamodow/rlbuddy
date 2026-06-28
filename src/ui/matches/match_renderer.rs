use super::core::{MatchInfo, MatchPlayer};
use crate::core::{Playlist, Rank};
use crate::rl::{EventRanks, Platform, RankAPI, Team, TeamScores};
use crate::ui::matches::widget::format_seconds;
use eframe::egui::{self, Color32};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::SystemTime;

pub struct MatchRenderer<'a> {
    match_info: &'a MatchInfo,
    ranks: &'a RankAPI,
}

impl<'a> MatchRenderer<'a> {
    pub fn new(match_info: &'a MatchInfo, ranks: &'a RankAPI) -> MatchRenderer<'a> {
        MatchRenderer { match_info, ranks }
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(finished) = &self.match_info.finish {
                if let Some(winner) = finished.winner.or_else(|| {
                    match self
                        .match_info
                        .score
                        .blue
                        .cmp(&self.match_info.score.orange)
                    {
                        Ordering::Greater => Some(Team::Blue),
                        Ordering::Less => Some(Team::Orange),
                        Ordering::Equal => None,
                    }
                }) {
                    ui.label(bold_text(if winner == self.match_info.our_team {
                        "Win"
                    } else {
                        "Loss"
                    }));
                }
            } else {
                ui.label("In progress");
            }

            score_labels(ui, &self.match_info.score, self.match_info.our_team);

            if let Some(finished) = &self.match_info.finish {
                let (text, refresh_in) = format_seconds(
                    SystemTime::now()
                        .duration_since(finished.timestamp)
                        .unwrap()
                        .as_secs(),
                );

                ui.label(text);
                ui.request_repaint_after(refresh_in);
            }
        });
    }

    fn render_player(
        &self,
        ui: &mut egui::Ui,
        playlist: Option<&Playlist>,
        match_player: &MatchPlayer,
    ) {
        let skill = if match_player.data.platform == Platform::Bot {
            None
        } else {
            self.ranks.get(&match_player.data.platform_id)
        };

        // rank in this gamemode
        if let Some(skill) = &skill
            && let Some(playlist) = playlist
        {
            MatchRenderer::render_player_rank_cell(ui, playlist, skill);
        } else {
            center_label(ui, "-");
        }

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;

            let name_color = if match_player.left {
                Color32::GRAY
            } else if match_player.data.is_self {
                ui.visuals().strong_text_color()
            } else {
                match match_player.data.team {
                    Team::Blue => Color32::from_rgb(64, 128, 255),
                    Team::Orange => Color32::ORANGE,
                }
            };
            ui.add(
                egui::Label::new(
                    bold_text(&match_player.data.name)
                        .color(name_color)
                        .size(15.0),
                )
                .extend(),
            );

            if let Some(skill) = &skill {
                MatchRenderer::render_rank_list(ui, match_player.left, skill);
            }
        });

        center_label(ui, match_player.data.score.to_string());
        ui.end_row();
    }

    fn render_player_rank_cell(ui: &mut egui::Ui, playlist: &Playlist, skill: &Arc<EventRanks>) {
        let rank = match playlist {
            Playlist::Ones => skill.duels.as_ref(),
            Playlist::Twos => skill.doubles.as_ref(),
            Playlist::Threes => skill.standard.as_ref(),
        };

        match rank {
            Some(rank) => {
                center_layout(ui, 28.0, |ui| {
                    if rank.rank_is_estimate {
                        ui.add(
                            egui::Image::new(Rank::Unranked.to_image())
                                .fit_to_exact_size(egui::vec2(28.0, 28.0)),
                        )
                        .on_hover_text(format!("Unranked in {playlist}"))
                    } else {
                        ui.add(
                            egui::Image::new(rank.rank.to_image())
                                .fit_to_exact_size(egui::vec2(28.0, 28.0)),
                        )
                        .on_hover_text(format!(
                            "{} rank: {}{}",
                            playlist,
                            rank.rank.as_str(),
                            rank.div
                        ))
                    }
                });
            }
            None => {
                center_label(ui, "-");
            }
        }
    }

    fn render_rank_list(ui: &mut egui::Ui, muted: bool, skill: &Arc<EventRanks>) {
        ui.horizontal(|ui| {
            let modes = [&skill.duels, &skill.doubles, &skill.standard];

            for mode in modes {
                // per-rank mmr + icon
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    if let Some(mode) = mode {
                        let image = ui.image(mode.rank.to_image());
                        if mode.rank_is_estimate {
                            image.on_hover_text("Estimated rank");
                        } else {
                            image.on_hover_text(
                                mode.rank.as_str().to_string() + &mode.div.to_string(),
                            );
                        }

                        if muted {
                            ui.label(mode.mmr.to_string());
                        } else {
                            ui.label(
                                egui::RichText::new(mode.mmr.to_string())
                                    .color(mode.rank.to_color()),
                            );
                        }
                    } else {
                        ui.image(Rank::Unranked.to_image());
                        ui.label(egui::RichText::new("---").color(Rank::Unranked.to_color()));
                    }
                });
            }
        });
    }
}

impl egui::Widget for MatchRenderer<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let playlist =
            Playlist::from_player_count(self.match_info.players.iter().filter(|p| !p.left).count());

        self.render_header(ui);

        egui::Grid::new(self.match_info.started_at)
            .spacing(egui::vec2(8.0, 12.0))
            .striped(true)
            .show(ui, |ui| {
                center_label(ui, bold_text("Rank"));
                ui.label(bold_text("Player"));
                center_label(ui, bold_text("Score"));

                ui.end_row();

                if self.match_info.finish.is_some() {
                    for player in filter_useless_bots(&self.match_info.players) {
                        self.render_player(ui, playlist.as_ref(), player);
                    }
                } else {
                    for player in &self.match_info.players {
                        self.render_player(ui, playlist.as_ref(), player);
                    }
                }
            })
            .response
    }
}

fn score_labels(ui: &mut egui::Ui, scores: &TeamScores, priority: Team) {
    let blue_text = egui::RichText::new(scores.blue.to_string()).color(Color32::LIGHT_BLUE);
    let orange_text = egui::RichText::new(scores.orange.to_string()).color(Color32::LIGHT_RED);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if priority == Team::Blue {
            ui.label(blue_text);
            ui.label("-");
            ui.label(orange_text);
        } else {
            ui.label(orange_text);
            ui.label("-");
            ui.label(blue_text);
        }
    });
}

fn center_layout<R>(
    ui: &mut egui::Ui,
    height: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        add_contents,
    )
}

fn center_label(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
) -> egui::InnerResponse<egui::Response> {
    center_layout(ui, 16.0, |ui| ui.label(text))
}

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
}

fn filter_useless_bots(players: &[MatchPlayer]) -> impl Iterator<Item = &MatchPlayer> {
    players
        .iter()
        .filter(|p| p.data.platform != Platform::Bot || p.data.score != 0)
}
