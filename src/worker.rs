use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use log::info;
use url::Url;

use crate::gemini::{transaction, Response};
use crate::state::{Event, Mode, State};
use crate::terminal;

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
            Event::Navigate(url_or_path) => {
                let url = {
                    let state = state.lock().expect("poisoned");

                    // Parse the URL to ensure it's valid and check if it has a base path
                    match Url::parse(&url_or_path) {
                        Ok(url) => url,
                        Err(url::ParseError::RelativeUrlWithoutBase) => {
                            // If we don't have a URL base, we clear the query/fragment and join
                            // on the requested path.
                            let mut url = state.current_url.as_ref().unwrap().clone();
                            url.set_query(None);
                            url.set_fragment(None);
                            url.join(&url_or_path).unwrap()
                        }
                        e => panic!("{:?}", e),
                    }
                };

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
            Event::Redraw => {
                let mut state = state.lock().expect("poisoned");

                // TODO: We don't always need to clear the screen. Only for things like scrolling.
                terminal::clear_screen().unwrap();

                state.render_page();
            }
            Event::TransactionComplete(response, url) => {
                let mut state = state.lock().expect("poisoned");

                match response {
                    Response::Body {
                        content,
                        status_code,
                    } => {
                        // Move the current line back to the top of the page
                        state.current_line_index = 0;

                        state.content = content;
                        state.current_url = Some(url);
                        state.last_status_code = Some(status_code);
                    }
                    Response::RedirectLoop(_url) => todo!("handle redirect loops"),
                }

                terminal::clear_screen().unwrap();
                state.render_page();
                state.mode = Mode::Normal;
            }
            Event::TransactionError(e) => {
                info!("transaction error: {}", e);

                let mut state = state.lock().expect("poisoned");

                state.set_error_message(e.to_string());
                terminal::clear_screen().unwrap();
                state.render_page();
                state.mode = Mode::Normal;
            }
            Event::Terminate => break,
        }
    }
}
