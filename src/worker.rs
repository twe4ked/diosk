use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use log::info;

use crate::state::{Event, State};

pub struct Worker;

impl Worker {
    pub fn spawn(state: Arc<Mutex<State>>, rx: mpsc::Receiver<Event>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            handle_event_loop(state, rx);
        })
    }
}

fn handle_event_loop(state: Arc<Mutex<State>>, rx: mpsc::Receiver<Event>) {
    loop {
        let event = rx.recv().unwrap();

        info!("event recv: {:?}", &event);

        match event {
            Event::TransactionComplete(response, url) => {
                let mut state = state.lock().expect("poisoned");
                state.transaction_complete(response, url);
            }
            Event::TransactionError(e) => {
                let mut state = state.lock().expect("poisoned");
                state.transaction_error(e);
            }
            Event::TerminateWorker => break,
        }
    }
}
