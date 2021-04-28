use crossterm::style::{Color::Reset, SetBackgroundColor as Bg};
use diosk::terminal::colors::all as all_colors;

fn main() {
    for (name, color) in all_colors() {
        println!("{}      {} {}", Bg(color), Bg(Reset), name);
    }
}
