use std::fmt;

use eframe::egui::{self, Color32};
use num_enum::{FromPrimitive, TryFromPrimitive};

#[derive(Debug, PartialEq, TryFromPrimitive)]
#[repr(u8)]
#[allow(dead_code)] // since its constructed with mem::transmute
pub enum Rank {
    Unranked,
    Bronze1,
    Bronze2,
    Bronze3,
    Silver1,
    Silver2,
    Silver3,
    Gold1,
    Gold2,
    Gold3,
    Plat1,
    Plat2,
    Plat3,
    Diamond1,
    Diamond2,
    Diamond3,
    Champ1,
    Champ2,
    Champ3,
    GC1,
    GC2,
    GC3,
    Ssl,
}

impl Rank {
    pub fn as_str(&self) -> &'static str {
        match self {
            Rank::Unranked => "Unranked",
            Rank::Bronze1 => "Bronze I",
            Rank::Bronze2 => "Bronze II",
            Rank::Bronze3 => "Bronze III",
            Rank::Silver1 => "Silver I",
            Rank::Silver2 => "Silver II",
            Rank::Silver3 => "Silver III",
            Rank::Gold1 => "Gold I",
            Rank::Gold2 => "Gold II",
            Rank::Gold3 => "Gold III",
            Rank::Plat1 => "Platinum I",
            Rank::Plat2 => "Platinum II",
            Rank::Plat3 => "Platinum III",
            Rank::Diamond1 => "Diamond I",
            Rank::Diamond2 => "Diamond II",
            Rank::Diamond3 => "Diamond III",
            Rank::Champ1 => "Champion I",
            Rank::Champ2 => "Champion II",
            Rank::Champ3 => "Champion III",
            Rank::GC1 => "Grand Champion I",
            Rank::GC2 => "Grand Champion II",
            Rank::GC3 => "Grand Champion III",
            Rank::Ssl => "Supersonic Legend",
        }
    }

    pub fn to_image(&self) -> egui::ImageSource<'static> {
        match self {
            Rank::Unranked => egui::include_image!("../../assets/Unranked_icon.png"),
            Rank::Bronze1 => egui::include_image!("../../assets/Bronze1_rank_icon.png"),
            Rank::Bronze2 => egui::include_image!("../../assets/Bronze2_rank_icon.png"),
            Rank::Bronze3 => egui::include_image!("../../assets/Bronze3_rank_icon.png"),
            Rank::Silver1 => egui::include_image!("../../assets/Silver1_rank_icon.png"),
            Rank::Silver2 => egui::include_image!("../../assets/Silver2_rank_icon.png"),
            Rank::Silver3 => egui::include_image!("../../assets/Silver3_rank_icon.png"),
            Rank::Gold1 => egui::include_image!("../../assets/Gold1_rank_icon.png"),
            Rank::Gold2 => egui::include_image!("../../assets/Gold2_rank_icon.png"),
            Rank::Gold3 => egui::include_image!("../../assets/Gold3_rank_icon.png"),
            Rank::Plat1 => egui::include_image!("../../assets/Platinum1_rank_icon.png"),
            Rank::Plat2 => egui::include_image!("../../assets/Platinum2_rank_icon.png"),
            Rank::Plat3 => egui::include_image!("../../assets/Platinum3_rank_icon.png"),
            Rank::Diamond1 => egui::include_image!("../../assets/Diamond1_rank_icon.png"),
            Rank::Diamond2 => egui::include_image!("../../assets/Diamond2_rank_icon.png"),
            Rank::Diamond3 => egui::include_image!("../../assets/Diamond3_rank_icon.png"),
            Rank::Champ1 => egui::include_image!("../../assets/Champion1_rank_icon.png"),
            Rank::Champ2 => egui::include_image!("../../assets/Champion2_rank_icon.png"),
            Rank::Champ3 => egui::include_image!("../../assets/Champion3_rank_icon.png"),
            Rank::GC1 => egui::include_image!("../../assets/Grand_Champion1_rank_icon.png"),
            Rank::GC2 => egui::include_image!("../../assets/Grand_Champion2_rank_icon.png"),
            Rank::GC3 => egui::include_image!("../../assets/Grand_Champion3_rank_icon.png"),
            Rank::Ssl => egui::include_image!("../../assets/Supersonic_Legend_rank_icon.png"),
        }
    }

    pub fn to_color(&self) -> Color32 {
        match self {
            Rank::Unranked => Color32::DARK_GRAY,
            Rank::Bronze1 | Rank::Bronze2 | Rank::Bronze3 => Color32::BROWN,
            Rank::Silver1 | Rank::Silver2 | Rank::Silver3 => Color32::GRAY,
            Rank::Gold1 | Rank::Gold2 | Rank::Gold3 => Color32::YELLOW,
            Rank::Plat1 | Rank::Plat2 | Rank::Plat3 => Color32::LIGHT_BLUE,
            Rank::Diamond1 | Rank::Diamond2 | Rank::Diamond3 => Color32::BLUE,
            Rank::Champ1 | Rank::Champ2 | Rank::Champ3 => Color32::PURPLE,
            Rank::GC1 | Rank::GC2 | Rank::GC3 => Color32::RED,
            Rank::Ssl => Color32::WHITE,
        }
    }

    // uses f2p season 23 1v1
    pub fn estimate_from_mmr(mmr: i16) -> Rank {
        #[allow(clippy::match_overlapping_arm)]
        match mmr {
            ..=156 => Rank::Bronze1,
            ..=213 => Rank::Bronze2,
            ..=274 => Rank::Bronze3,
            ..=334 => Rank::Silver1,
            ..=394 => Rank::Silver2,
            ..=454 => Rank::Silver3,
            ..=514 => Rank::Gold1,
            ..=574 => Rank::Gold2,
            ..=634 => Rank::Gold3,
            ..=694 => Rank::Plat1,
            ..=753 => Rank::Plat2,
            ..=808 => Rank::Plat3,
            ..=874 => Rank::Diamond1,
            ..=930 => Rank::Diamond2,
            ..=994 => Rank::Diamond3,
            ..=1052 => Rank::Champ1,
            ..=1114 => Rank::Champ2,
            ..=1170 => Rank::Champ3,
            ..=1232 => Rank::GC1,
            ..=1295 => Rank::GC2,
            ..=1351 => Rank::GC3,
            _ => Rank::Ssl,
        }
    }
}

#[derive(Debug, FromPrimitive)]
#[repr(u8)]
pub enum Division {
    #[num_enum(default)]
    None,
    One,
    Two,
    Three,
    Four,
}

impl fmt::Display for Division {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Division::None => "",
                Division::One => " Div I",
                Division::Two => " Div II",
                Division::Three => " Div III",
                Division::Four => " Div IV",
            }
        )
    }
}
