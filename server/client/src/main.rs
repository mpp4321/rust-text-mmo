use std::{sync::Arc, ops::Range, io::Read};

use fltk::{
    app,
    enums::{Color, Event, Shortcut, Font, Key},
    prelude::{DisplayExt, GroupExt, WidgetBase, WidgetExt},
    text::{SimpleTerminal, StyleTableEntry, TextBuffer},
    utils,
    window::Window, browser::FileBrowser, dialog,
};

use lazy_static::lazy_static;
use regex::{Regex, Captures};
use tokio::{sync::Mutex, net::TcpSocket, io::{BufReader, BufWriter, AsyncWriteExt, AsyncBufReadExt}};

const WIDTH: i32 = 1024;
const HEIGHT: i32 = 640;

pub trait TerminalFuncs {
    fn append_txt(&mut self, txt: &str);
    fn append_colored(&mut self, txt: &str, color_buf: &str);
    fn run_command(&mut self, cmd: &str, receiver: app::Receiver<bool>, queue: Arc<Mutex<Vec<String>>>);
}

impl TerminalFuncs for SimpleTerminal {
    fn append_txt(&mut self, txt: &str) {
        lazy_static! {
            static ref REGEX_TXT: Regex = Regex::new("@([A-Z])(.+)@").unwrap();
        }
        if REGEX_TXT.is_match(txt) {
            let matches = REGEX_TXT.captures_iter(&txt).collect::<Vec<Captures>>();
            let mut txt: String = txt.into();
            let mut mods: Vec<(Range<usize>, String)> = vec![];
            let start_index = self.style_buffer().unwrap().length();
            println!("{}", txt);
            for cap in matches.iter() {
                let full_in_text = cap.get(0).unwrap();
                println!("{}", full_in_text.as_str());
                let text_ful = cap.get(2).unwrap().as_str();
                let type_text = cap.get(1).unwrap().as_str();
                txt = txt.replace(full_in_text.as_str(), text_ful);
                mods.push((full_in_text.start()..(full_in_text.start() + text_ful.len()), type_text.into()));
            }
            println!("{}", txt);
            self.append(&txt);
            self.style_buffer().unwrap().append(&"A".repeat(txt.len()));
            for mod_txt in mods {
                let len = mod_txt.0.len();
                self.style_buffer().unwrap().replace(mod_txt.0.start as i32 + start_index , mod_txt.0.end as i32 + start_index, mod_txt.1.repeat(len).as_str());
            }
        } else {
            self.append(txt);
            self.style_buffer().unwrap().append(&"A".repeat(txt.len()));
        }
    }

    fn append_colored(&mut self, dir: &str, color_buf: &str) {
        self.append(dir);
        self.style_buffer().unwrap().append(&color_buf.repeat(dir.len()));
    }

    fn run_command(&mut self, cmd: &str, _: app::Receiver<bool>, queue: Arc<Mutex<Vec<String>>>) {
        let mut cmd: String = format!("{}", cmd);

        // If we are sending a script open file dialog pause until responds and then send contents
        // in format of \script script_name:{file contents} then server will parse it and save
        if cmd.starts_with("\\script") {
            let mut chooser = dialog::FileChooser::new(
                ".",
                "*",
                dialog::FileChooserType::Single,
                "Choose a script"
            );
            chooser.show();
            chooser.window().set_pos(300, 300);

            while chooser.shown() {
                app::wait();
            }

            if let Some(file) = chooser.value(1) {
                let mut file_contents = String::new();
                let _ = std::io::BufReader::new(std::fs::File::open(file).unwrap()).read_to_string(&mut file_contents).expect("Failed to read file");
                // Server terminating char is new line so we replace with #n and then parse on
                // server
                cmd = format!("{}{}", cmd, file_contents).replace("\n", "#n");
            } else {
                return;
            }
        }


        tokio::spawn(async move {
            // Server terminating char is new line since we read_line
            queue.lock().await.push(format!{"{}\n", cmd});
        });
    }

}

#[derive(Debug, Clone)]
struct Term {
    #[allow(dead_code)]
    term: Arc<Mutex<SimpleTerminal>>
}

impl Term {
    pub fn get_ref_of_term(&self) -> Arc<Mutex<SimpleTerminal>> {
        return self.term.clone();
    }

    pub async fn new(queued_messages: Arc<Mutex<Vec<String>>>) -> std::io::Result<Term> {
        let mut cmd = String::new();

        // Enable different colored text in TestDisplay
        let styles: Vec<StyleTableEntry> = vec![
            StyleTableEntry {
                color: Color::DarkYellow,
                font: Font::Courier,
                size: 16,
            },
            StyleTableEntry {
                color: Color::Red,
                font: Font::Courier,
                size: 16,
            },
            StyleTableEntry {
                color: Color::Blue,
                font: Font::Courier,
                size: 16,
            },
            StyleTableEntry {
                color: Color::DarkYellow,
                font: Font::CourierBold,
                size: 16,
            },
            StyleTableEntry {
                color: Color::from_u32(0x8000ff),
                font: Font::Courier,
                size: 16,
            },
        ];

        let mut sbuf = TextBuffer::default();
        let term = Arc::new(Mutex::new(SimpleTerminal::new(5, 5, WIDTH - 10, HEIGHT - 10, "")));
        let mut term_ref = term.lock().await;

        term_ref.set_highlight_data(sbuf.clone(), styles);

        let mut curr = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        curr.push_str("$ ");

        let (s, r) = app::channel();
        let queued_messages_closure = queued_messages.clone();

        term_ref.handle(move |t, ev| {
            let queued_messages = queued_messages_closure.clone();
            match ev {
                Event::KeyDown => match app::event_key() {
                    Key::Enter => {
                        t.append_txt("\n");
                        t.run_command(&cmd, r, queued_messages.clone());
                        cmd.clear();
                        true
                    }
                    Key::BackSpace => {
                        if !cmd.is_empty() {
                            let c = cmd.pop().unwrap();
                            let len = if c.is_ascii() {
                                1
                            } else {
                                utils::char_len(c) as i32
                            };
                            let text_len = t.text().len() as i32;
                            t.buffer().unwrap().remove(text_len - len, text_len);
                            sbuf.remove(text_len - len, text_len);
                            true
                        } else {
                            false
                        }
                    }
                    _ => {
                        if let Some(ch) = app::event_text().chars().next() {
                            if app::compose().is_some() {
                                let temp = ch.to_string();
                                cmd.push_str(&temp);
                                t.append_txt(&temp);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                },
                Event::KeyUp => {
                    if app::event_state() == Shortcut::Ctrl && app::event_key() == Key::from_char('c') {
                        s.send(true);
                    }
                    false
                }
                _ => false,
            }
        });
        drop(term_ref);
        Ok(Self { term })
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let queued_messages = Arc::new(Mutex::new(Vec::<String>::new()));

    let socket = TcpSocket::new_v4()?;
    let stream = socket.connect("127.0.0.1:8080".parse().unwrap()).await?;

    let app = app::App::default().with_scheme(app::Scheme::Plastic);
    let mut wind = Window::default()
        .with_size(WIDTH, HEIGHT)
        .with_label("Mikey Realm");

    let _term = Term::new(queued_messages.clone()).await.unwrap();
    let term_ref = _term.get_ref_of_term();

    tokio::spawn(async move {
        let (read, write) = stream.into_split();
        tokio::spawn(async move {
            let term_ref_reader = term_ref.clone();
            let mut reader = BufReader::new(read);
            loop {
                let mut string_buf = String::new();
                reader.read_line(&mut string_buf).await.expect("Failed to read buffer");
                let mut term_instance = term_ref_reader.lock().await;
                term_instance.append_txt(&string_buf);
            }
        });
        let mut writer = BufWriter::new(write);
        loop {
            let mut value = queued_messages.lock().await;
            for message in value.iter() {
                writer.write(message.as_bytes()).await.expect("Write error");
            }
            value.clear();
            writer.flush().await.expect("Failed to send");
        }
    });

    wind.make_resizable(true);
    wind.end();
    wind.show();

    app.run().unwrap();
    Ok(())
}
