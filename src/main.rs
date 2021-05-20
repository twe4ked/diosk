use std::sync::{Arc, Mutex};

use diosk::input::run as run_input_loop;
use diosk::state::State;
use diosk::terminal;
use diosk::worker::Worker;

//  ,ogggggggg,
// dP"""88""""Y8b,                          ,dPYb,
// Yb,  88     `8b,                         IP'`Yb
//  `"  88     `8b'gg                       I8  8I
//      88      d8'""    ,ggggg,    ,g,     I8 dP" "8
//      88     ,8P 88   dP"  "Y8ggg,8'8,    I8d8bggP"
//      88___,dP'_,88_,d8,   ,d8',8'_   8) ,d8    `Yb,
//     888888P"  8P""YP"Y8888P"  P' "YY8P8P88P      Y8

fn main() {
    simple_logging::log_to_file("target/out.log", log::LevelFilter::Info)
        .expect("unable to set up logging");

    // Enhance the panic hook to handle re-setting the terminal
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        terminal::teardown().expect("unable to reset terminal");
        default_panic(info);

        // Ensure the process is exited if a thread panics
        std::process::exit(1);
    }));

    terminal::setup_alternate_screen().expect("unable to setup terminal");

    // Initialize State
    let (state, tx, rx) = {
        let (state, tx, rx) = State::new();
        state.send_redraw();
        (Arc::new(Mutex::new(state)), tx, rx)
    };

    // Spawn the worker thread
    let worker = Worker::spawn(state.clone(), tx, rx);

    // Run a blocking input loop
    run_input_loop(state);

    // Wait for the worker thread to finish
    worker.join().expect("worker thread panicked");

    // Clean up the terminal
    terminal::teardown().expect("unable to reset terminal");
}
