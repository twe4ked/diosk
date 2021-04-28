use std::fmt;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crossterm::event::{read, Event as TermEvent, KeyCode};
use log::{info, LevelFilter};
use url::Url;

use diosk::gemini::gemtext::Line;
use diosk::gemini::{transaction, Response, StatusCode};
use diosk::terminal::Terminal;

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

#[derive(Debug)]
enum Event {
    Navigate(String),
    Terminate,
    Redraw,
}

#[derive(Debug, Clone)]
enum Mode {
    Normal,
    Loading,
    Input,
}

struct State {
    current_line: usize,
    content: String,
    mode: Mode,
    tx: mpsc::Sender<Event>,
    current_url: Url,
    last_status_code: StatusCode,
    terminal: Terminal,
}

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut content = self.content.clone();
        content.truncate(10);

        fmt.debug_struct("State")
            .field("current_line", &self.current_line)
            .field("mode", &self.mode)
            .field("current_url", &self.current_url.to_string())
            .finish()
    }
}

impl State {
    fn line(&self, index: usize) -> &str {
        self.content
            .lines()
            .nth(index as usize)
            .expect("current line not found")
    }

    fn current_line(&self) -> &str {
        self.line(self.current_line)
    }

    fn down(&mut self) {
        self.current_line += 1;
        self.tx.send(Event::Redraw).unwrap();
    }

    fn up(&mut self) {
        self.current_line -= 1;
        self.tx.send(Event::Redraw).unwrap();
    }

    fn go(&mut self) {
        self.mode = Mode::Input;
        todo!();
    }

    fn quit(&mut self) {
        self.tx.send(Event::Terminate).unwrap();
    }

    fn enter(&mut self) {
        let line = self.current_line();

        if let Line::Link { url, .. } = Line::parse(line) {
            // Navigate
            self.mode = Mode::Loading;
            self.tx.send(Event::Navigate(url)).unwrap();
        } else {
            // Nothing to do on non-link lines
        }
    }

    fn render_page(&mut self) {
        let content = self.content.clone();
        let current_line = self.current_line;
        let current_url = self.current_url.clone();
        let last_status_code = self.last_status_code.clone();

        self.terminal
            .render_page(current_line, content, &current_url, last_status_code)
            .unwrap();
    }
}

fn main() {
    simple_logging::log_to_file("target/out.log", LevelFilter::Info).unwrap();

    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        Terminal::teardown().unwrap();
        default_panic(info);
    }));

    let initial_url = Url::parse("gemini://gemini.circumlunar.space/software/").unwrap();
    let (initial_content, last_status_code) =
        match transaction(&initial_url, 0).expect("initial transaction failed") {
            Response::Body {
                content,
                status_code,
            } => (content.unwrap(), status_code),
            _ => panic!("initial URL must contain a body"),
        };

    let (tx, rx) = mpsc::channel::<Event>();

    let terminal = Terminal::setup_alternate_screen().unwrap();

    let state = State {
        current_line: 0,
        content: initial_content,
        current_url: initial_url,
        last_status_code,
        mode: Mode::Normal,
        tx,
        terminal,
    };
    let state_mutex = Arc::new(Mutex::new(state));

    let worker = {
        let state_mutex = state_mutex.clone();

        thread::spawn(move || {
            handle_event_loop(state_mutex, rx);
        })
    };

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

                state.terminal.flush().unwrap();

                info!("{:?}", &state);
            }
            TermEvent::Mouse(event) => info!("{:?}", event),
            TermEvent::Resize(width, height) => info!("New size {}x{}", width, height),
        }
    }

    // Wait for the worker thread to finish
    worker.join().unwrap();

    // Clean up the terminal
    Terminal::teardown().unwrap();
}

fn handle_event_loop(state_mutex: Arc<Mutex<State>>, rx: mpsc::Receiver<Event>) {
    loop {
        let event = rx.recv().unwrap();

        match event {
            Event::Navigate(url_or_path) => {
                let mut state = state_mutex.lock().expect("poisoned");

                // Parse the URL to ensure it's valid and check if it has a base path
                let url = match Url::parse(&url_or_path) {
                    Ok(url) => url,
                    Err(url::ParseError::RelativeUrlWithoutBase) => {
                        // If we don't have a URL base, we clear the query/fragment and join
                        // on the requested path.
                        let mut url = state.current_url.clone();
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
                            state.content = content.unwrap();
                            state.current_url = url;
                            state.last_status_code = status_code;
                        }
                        Response::RedirectLoop(_url) => todo!("handle redirect loops"),
                    },
                    Err(_) => {
                        info!("transaction error");

                        state.mode = Mode::Normal;
                        continue;
                    }
                }

                // Move the current line back to the top of the page
                state.current_line = 0;

                state.terminal.clear_screen().unwrap();

                state.render_page();

                state.mode = Mode::Normal;
            }
            Event::Redraw => {
                let mut state = state_mutex.lock().expect("poisoned");
                state.render_page();
            }
            Event::Terminate => break,
        }
    }
}
