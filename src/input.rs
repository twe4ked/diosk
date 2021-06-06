use std::sync::{Arc, Mutex};

use crossterm::event::{read, Event, KeyCode, KeyEvent};
use log::info;

use crate::state::input::InputEnterResult;
use crate::state::{Mode, State};

mod edit;

use edit::Command;

pub fn run(state: Arc<Mutex<State>>) {
    loop {
        let event = read().unwrap();
        let mut state = state.lock().expect("poisoned");

        match event {
            Event::Key(event) => handle_key_event(&mut state, event),
            Event::Mouse(event) => info!("{:?}", event),
            Event::Resize(width, height) => state.new_size(width, height),
        }

        if state.terminated() {
            break;
        }
    }
}

fn handle_key_event(state: &mut State, event: KeyEvent) {
    state.clear_error_message();

    match state.mode() {
        Mode::Normal | Mode::Loading => match event.code {
            KeyCode::Char(':') => state.input(),
            KeyCode::Char('j') => state.down(),
            KeyCode::Char('k') => state.up(),
            KeyCode::Enter => {
                if matches!(state.mode(), Mode::Loading) {
                    state.loading_mode_enter();
                } else {
                    state.enter();
                }
            }
            _ => {}
        },

        Mode::Input => {
            if let Some(command) = edit::command(event) {
                match command {
                    Command::DeleteWord => {
                        state.input.delete_word();
                        state.clear_screen_and_render_page();
                    }
                    Command::DeleteChar => {
                        state.input.delete_char();
                        state.clear_screen_and_render_page();
                    }
                    Command::AddChar(c) => {
                        state.input.input_char(c);
                        state.clear_screen_and_render_page();
                    }
                    Command::Enter => match state.input.enter() {
                        InputEnterResult::Navigate(url) => {
                            state.request(&url);
                            state.clear_screen_and_render_page();
                        }
                        InputEnterResult::Quit => {
                            state.quit();
                        }
                        InputEnterResult::Invalid(input) => {
                            state.mode = Mode::Normal;
                            state.set_error_message(format!("Invalid command: {}", input));
                            state.clear_screen_and_render_page();
                        }
                    },
                    Command::Esc => {
                        state.input.cancel();
                        state.mode = Mode::Normal;
                        state.clear_screen_and_render_page();
                    }
                }
            }
        }
    }

    info!("{:?}", &state);
}
