use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use log::info;
use url::Url;

use crate::gemini::{transaction, Response};
use crate::state::{Event, Mode, State};
use crate::terminal;

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

        match event {
            Event::Navigate(url_or_path) => {
                let mut state = state.lock().expect("poisoned");

                // Parse the URL to ensure it's valid and check if it has a base path
                let url = match Url::parse(&url_or_path) {
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
                };

                info!("navigating to: {}", &url);

                match transaction(&url, 0) {
                    Ok(response) => match response {
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
                    },
                    Err(e) => {
                        info!("transaction error: {}", e);
                        state.set_error_message(e.to_string());
                    }
                }

                terminal::clear_screen().unwrap();

                state.render_page();

                state.mode = Mode::Normal;
            }
            Event::Redraw => {
                let mut state = state.lock().expect("poisoned");

                // TODO: We don't always need to clear the screen. Only for things like scrolling.
                terminal::clear_screen().unwrap();

                state.render_page();
            }
            Event::Terminate => break,
        }
    }
}
