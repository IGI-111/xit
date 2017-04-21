extern crate termion;
#[macro_use]
extern crate serde_json;

mod core;

use termion::raw::IntoRawMode;
use termion::input::{TermRead, MouseTerminal};
use termion::event::{Event, Key};
use termion::screen::AlternateScreen;
use termion::cursor;
use termion::clear;
use termion::style;
use std::time::{Instant, Duration};
use std::sync::mpsc::{self, Receiver};
use std::io::{stdin, stdout, Write, Read, BufWriter};
use core::{Update, Core};
use std::thread;
use std::str::FromStr;

fn main() {
    let event_thread;
    {
        let (mut core, updates) = Core::new();

        event_thread = thread::spawn(move || update_loop(updates));

        let ref view_id = core.new_view("");
        core.scroll(view_id, (0,10));

        let stdin = stdin();
        for e in stdin.events() {
            match e {
                Ok(Event::Key(Key::Ctrl('q'))) => break,
                Ok(Event::Key(Key::Char(c))) => core.insert(view_id, &c.to_string()),
                Ok(Event::Key(Key::Right)) => core.move_right(view_id),
                Ok(Event::Key(Key::Left)) => core.move_left(view_id),
                Ok(Event::Key(Key::Up)) => core.move_up(view_id),
                Ok(Event::Key(Key::Down)) => core.move_down(view_id),
                _ => {}
            }
        }
    }
    event_thread.join().unwrap();
}

fn update_loop(updates: Receiver<Update>) {
    let mut stdout = BufWriter::new(MouseTerminal::from(AlternateScreen::from(stdout().into_raw_mode().unwrap())));

    write!(stdout, "{}{}", clear::All, cursor::Hide).unwrap();
    for event in updates.iter() {
        handle_event(&mut stdout, event);
    }
    write!(stdout, "{}", cursor::Show).unwrap();
}

fn handle_event<W:Write>(stdout: &mut W, e: Update) {
    match e.get("method").unwrap().as_str().unwrap() {
        "update" => {
            write!(stdout, "{}", cursor::Goto(1, 1)).unwrap();
            let ops = e.get("params").unwrap().get("update").unwrap().get("ops").unwrap().as_array().unwrap();
            for op in ops {
                if let Some(lines) = op.get("lines") {
                    for (i, line) in lines.as_array().unwrap().iter().enumerate() {
                        write!(stdout, "{}{}", cursor::Goto(1, 1 + i as u16), clear::CurrentLine).unwrap();

                        if let Some(text) = line.get("text") {
                            let mut line_text = String::from_str(text.as_str().unwrap()).unwrap();
                            pad_line(&mut line_text);

                            if let Some(cursor) = line.get("cursor") {
                                for index in cursor.as_array().unwrap() {
                                    show_cursor(index.as_u64().unwrap() as usize, &mut line_text);
                                }
                            }

                            write!(stdout, "{}{}", line_text, style::Reset).unwrap();
                        }
                        write!(stdout, "{}", clear::AfterCursor).unwrap();
                    }
                    stdout.flush().unwrap();
                }
            }
        }
        "scroll_to" => {
            let params = e.get("params").unwrap();
            println!("{}", params);
            // let col = params.get("col").unwrap().as_u64().unwrap() as u16 + 1;
            // let line = params.get("line").unwrap().as_u64().unwrap() as usize;
        }
        _ => {}
    }
}

fn pad_line(text: &mut String) {
    if let Some('\n') = text.chars().last() {
        text.pop();
        text.push_str(" \n");
    } else {
        text.push(' ');
    }
}

fn show_cursor(cursor_index: usize, text: &mut String) {
    text.insert_str(cursor_index+1, &format!("{}", style::Reset));
    text.insert_str(cursor_index, &format!("{}", style::Invert));
}
