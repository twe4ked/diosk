use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use log::info;

use crate::gemini::transaction;
use crate::state::{Event, State};

pub struct Worker;

impl Worker {
    pub fn spawn(
        state: Arc<Mutex<State>>,
        tx: mpsc::Sender<Event>,
        rx: mpsc::Receiver<Event>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            handle_event_loop(state, tx, rx);
        })
    }
}

fn handle_event_loop(state: Arc<Mutex<State>>, tx: mpsc::Sender<Event>, rx: mpsc::Receiver<Event>) {
    loop {
        let event = rx.recv().unwrap();

        info!("event recv: {:?}", &event);

        match event {
            Event::Navigate(url) => {
                info!("navigating to: {}", &url);

                let tx = tx.clone();
                thread::spawn(move || {
                    let response = match transaction(&url) {
                        Ok(response) => tx.send(Event::TransactionComplete(response, url)),
                        Err(e) => tx.send(Event::TransactionError(e)),
                    };

                    info!("finished navigating");

                    response
                });
            }
            Event::TransactionComplete(response, url) => {
                let mut state = state.lock().expect("poisoned");
                state.transaction_complete(response, url);
            }
            Event::TransactionError(e) => {
                let mut state = state.lock().expect("poisoned");
                state.transaction_error(e);
            }
            Event::Terminate => break,
        }
    }
}
