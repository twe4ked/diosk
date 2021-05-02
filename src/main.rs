use std::sync::{mpsc, Arc, Mutex};

use crossterm::event::{read, Event as TermEvent, KeyCode};
use log::{info, LevelFilter};

use diosk::state::{Event, Mode, State};
use diosk::terminal::{self, Terminal};
use diosk::worker::Worker;

//  ,gggggggggg,
// dP"""88""""Y8b,                           ,dPYb,
// Yb,  88     `8b,                          IP'`Yb
//  `"  88      `8b gg                       I8  8I
//      88       Y8 ""                       I8  8bgg,
//      88       d8 gg    ,ggggg,    ,g,     I8 dP" "8
//      88      ,8P 88   dP"  "Y8ggg,8'8,    I8d8bggP"
//      88     ,8P' 88  i8'    ,8I ,8'  Yb   I8P' "Yb,
//      88____,dP'_,88_,d8,   ,d8',8'_   8) ,d8    `Yb,
//     8888888P"  8P""YP"Y8888P"  P' "YY8P8P88P      Y8

fn main() {
    simple_logging::log_to_file("target/out.log", LevelFilter::Info).unwrap();

    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        terminal::teardown().unwrap();
        default_panic(info);
    }));

    let (tx, rx) = mpsc::sync_channel::<Event>(32);

    terminal::setup_alternate_screen().unwrap();

    let mut state = State::new(tx);

    state.request("gemini://gemini.circumlunar.space/software/".to_string());

    let state_mutex = Arc::new(Mutex::new(state));

    let worker = Worker::run(state_mutex.clone(), rx);

    // Draw the initial page
    {
        let mut state = state_mutex.lock().expect("poisoned");
        state.render_page();
    }

    loop {
        match read().unwrap() {
            TermEvent::Key(event) => {
                let mut state = state_mutex.lock().expect("poisoned");
                let mode = state.mode.clone();

                match mode {
                    Mode::Loading => {
                        if let KeyCode::Char('q') = event.code {
                            state.quit();
                            break;
                        }
                    }

                    Mode::Normal => match event.code {
                        KeyCode::Char('q') => {
                            state.quit();
                            break;
                        }
                        KeyCode::Char('g') => state.go(),
                        KeyCode::Char('j') => state.down(),
                        KeyCode::Char('k') => state.up(),
                        KeyCode::Enter => state.enter(),
                        _ => {}
                    },

                    Mode::Input => todo!(),
                }

                Terminal::flush().unwrap();

                info!("{:?}", &state);
            }
            TermEvent::Mouse(event) => info!("{:?}", event),
            TermEvent::Resize(width, height) => info!("New size {}x{}", width, height),
        }
    }

    // Wait for the worker thread to finish
    worker.join().unwrap();

    // Clean up the terminal
    terminal::teardown().unwrap();
}
