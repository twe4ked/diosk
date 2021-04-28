#![allow(dead_code)]

// https://github.com/metalelf0/jellybeans-nvim/blob/cef41133874073b35bf7e8061d97a5214623770d/lua/lush_theme/jellybeans-nvim.lua#L48

use crossterm::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

pub const FOREGROUND: Color = rgb(232, 232, 211);
pub const BACKGROUND: Color = rgb(21, 21, 21);
pub const GREY: Color = rgb(136, 136, 136);
pub const GREY_ONE: Color = rgb(28, 28, 28);
pub const GREY_TWO: Color = rgb(240, 240, 240);
pub const GREY_THREE: Color = rgb(51, 51, 51);
pub const REGENT_GREY: Color = rgb(144, 152, 160);
pub const SCORPION: Color = rgb(96, 96, 96);
pub const COD_GREY: Color = rgb(16, 16, 16);
pub const TUNDORA: Color = rgb(64, 64, 64);
pub const ZAMBEZI: Color = rgb(96, 89, 88);
pub const SILVER_RUST: Color = rgb(204, 197, 196);
pub const SILVER: Color = rgb(199, 199, 199);
pub const ALTO: Color = rgb(221, 221, 221);
pub const GRAVEL: Color = rgb(64, 60, 65);
pub const BOULDER: Color = rgb(119, 119, 119);
pub const COCOA_BROWN: Color = rgb(48, 32, 40);
pub const GREY_CHATEAU: Color = rgb(160, 168, 176);
pub const BRIGHT_GREY: Color = rgb(56, 64, 72);
pub const SHUTTLE_GREY: Color = rgb(83, 93, 102);
pub const MINE_SHAFT: Color = rgb(31, 31, 31);
pub const TEMPTRESS: Color = rgb(64, 0, 10);
pub const BAYOUX_BLUE: Color = rgb(85, 103, 121);
pub const TOTAL_WHITE: Color = rgb(255, 255, 255);
pub const TOTAL_BLACK: Color = rgb(0, 0, 0);
pub const CADET_BLUE: Color = rgb(176, 184, 192);
pub const PERANO: Color = rgb(176, 208, 240);
pub const WEWAK: Color = rgb(240, 160, 192);
pub const MANTIS: Color = rgb(112, 185, 80);
pub const RAW_SIENNA: Color = rgb(207, 106, 76);
pub const HIGHLAND: Color = rgb(121, 157, 106);
pub const HOKI: Color = rgb(102, 135, 153);
pub const GREEN_SMOKE: Color = rgb(153, 173, 106);
pub const COSTA_DEL_SOL: Color = rgb(85, 102, 51);
pub const BILOBA_FLOWER: Color = rgb(198, 182, 238);
pub const MORNING_GLORY: Color = rgb(143, 191, 220);
pub const GOLDENROD: Color = rgb(250, 208, 122);
pub const SHIP_COVE: Color = rgb(129, 151, 191);
pub const KOROMIKO: Color = rgb(255, 185, 100);
pub const BRANDY: Color = rgb(218, 208, 133);
pub const OLD_BRICK: Color = rgb(144, 32, 32);
pub const DARK_BLUE: Color = rgb(0, 0, 223);
pub const RIPE_PLUM: Color = rgb(84, 0, 99);
pub const CASAL: Color = rgb(45, 112, 103);
pub const PURPLE: Color = rgb(112, 0, 137);
pub const TEA_GREEN: Color = rgb(210, 235, 190);
pub const DELL: Color = rgb(67, 112, 25);
pub const CALYPSO: Color = rgb(43, 91, 119);
