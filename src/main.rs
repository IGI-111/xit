extern crate termion;
#[macro_use]
extern crate serde_json;
extern crate futures;

mod core;

// use termion::raw::IntoRawMode;
// use termion::input::MouseTerminal;
// use std::io::{stdin, stdout};
use core::{Update, Core};
use std::thread;

fn main() {
    // let stdin = stdin();
    // let stdout = MouseTerminal::from(stdout().into_raw_mode().unwrap());

    let (mut core, events) = Core::new();
    thread::spawn(move || {
        for e in events {
            handle_event(e.unwrap());
        }
    });

    let ref view_id = core.new_view("");
    println!("{}", view_id);
    core.insert(view_id, "A");
    loop {
        thread::sleep_ms(10000);
    }


}

fn handle_event(e: Update) {
    println!("{}", e);
}
