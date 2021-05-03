use std::sync::{mpsc, Arc, Mutex};

use diosk::input::run as run_input_loop;
use diosk::state::{Event, State};
use diosk::terminal;
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
    simple_logging::log_to_file("target/out.log", log::LevelFilter::Info).unwrap();

    // Enhance the panic hook to handle re-setting the terminal
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        terminal::teardown().unwrap();
        default_panic(info);

        // Ensure the process is exited if a thread panics
        std::process::exit(1);
    }));

    terminal::setup_alternate_screen().unwrap();

    // Set up a channel for State to talk to the worker thread
    let (tx, rx) = mpsc::channel::<Event>();

    // Initialize State
    let state = {
        let mut state = State::new(tx);

        // Request and render the initial page
        state.request("gemini://gemini.circumlunar.space/software/".to_string());
        state.render_page();

        Arc::new(Mutex::new(state))
    };

    // Spawn the worker thread
    let worker = Worker::spawn(state.clone(), rx);

    // Run a blocking input loop
    run_input_loop(state);

    // Wait for the worker thread to finish
    worker.join().unwrap();

    // Clean up the terminal
    terminal::teardown().unwrap();
}
