#![allow(dead_code)]

use std::thread;
use futures::sync::{mpsc, oneshot};
use std::sync::{Arc, Mutex};
use futures::stream::{self, Stream};
use futures::Future;
use std::collections::HashMap;
use serde_json;
use std::process::{Stdio, Command, ChildStdin, ChildStdout};
use std::io::{self, BufReader, BufRead, Write};

pub type Update = serde_json::Value;
pub type Response = serde_json::Value;
pub type EventIterator = stream::Wait<mpsc::UnboundedReceiver<Update>>;

pub struct Core {
    rpc_tx_map: Arc<Mutex<HashMap<u64, oneshot::Sender<Response>>>>,
    rpc_index: u64,
    stdin: ChildStdin,
}

impl Core {
    pub fn new() -> (Core, EventIterator) {
        let process = Command::new("xi-core")
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not spawn core process, xi-core must be in your PATH.");

        let (update_tx, update_rx) = mpsc::unbounded();
        let rpc_tx_map = Arc::new(Mutex::new(HashMap::new()));
        let rpc_tx_map_clone = rpc_tx_map.clone();

        let stdout = process.stdout.unwrap();
        thread::spawn(move || { Self::event_loop(rpc_tx_map_clone, stdout, update_tx); });

        let stderr = process.stderr.unwrap();
        thread::spawn(move || {
                          let buf_reader = BufReader::new(stderr);
                          for line in buf_reader.lines() {
                              if let Ok(line) = line {
                                  writeln!(io::stderr(), "[core] {}", line).unwrap();
                              }
                          }
                      });
        (Core {
             rpc_tx_map: rpc_tx_map,
             rpc_index: 0,
             stdin: process.stdin.unwrap(),
         },
         update_rx.wait())
    }

    fn event_loop(rpc_tx_map: Arc<Mutex<HashMap<u64, oneshot::Sender<Response>>>>,
                  stdout: ChildStdout,
                  update_tx: mpsc::UnboundedSender<Update>) {
        for line in BufReader::new(stdout).lines() {
            if let Ok(data) = serde_json::from_str(line.unwrap().as_str()) {
                let req: serde_json::Value = data;

                if let (Some(id), Some(result)) = (req.get("id"), req.get("result")) {
                    // core request is an update
                    let mut map = rpc_tx_map.lock().unwrap();
                    let rpc_tx = map.remove(&id.as_u64().unwrap()).unwrap();
                    rpc_tx.send(result.clone()).unwrap();
                } else if let (Some(_), Some(_)) = (req.get("method"), req.get("params")) {
                    // core request is a response
                    update_tx.send(req.clone()).unwrap();
                } else {
                    panic!("Could not parse the core output: {:?}", req);
                }
            }


        }
    }

    /// Serialize JSON object and send it to the server
    fn send(&mut self, message: serde_json::Value) {
        let mut str_msg = serde_json::to_string(&message).unwrap();
        str_msg.push('\n');
        self.stdin.write(str_msg.as_bytes()).unwrap();
    }

    /// Build and send a JSON RPC request, returning the associated request ID to pair it with
    /// the response
    pub fn request(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        self.rpc_index += 1;
        let message = json!({
            "id": self.rpc_index,
            "method": method,
            "params": params
        });

        let (rpc_tx, rpc_rx) = oneshot::channel();
        {
            let mut map = self.rpc_tx_map.lock().unwrap();
            map.insert(self.rpc_index, rpc_tx);
        }

        self.send(message);

        rpc_rx.wait().unwrap()
    }

    /// Build and send a JSON RPC notification. No response is expected.
    fn notify(&mut self, method: &str, params: serde_json::Value) {
        let message = json!({
            "method": method,
            "params": params
        });
        self.send(message);
    }


    pub fn new_view(&mut self, filename: &str) -> String {
        let res = self.request("new_view", json!({"filename": filename}));
        res.as_str().unwrap().to_owned()
    }

    pub fn close_view(&mut self, view_id: &str) {
        self.notify("close_view", json!({"view-id": view_id}));
    }

    pub fn save(&mut self, view_id: &str, filename: &str) {
        self.notify("save", json!({"view-id": view_id, "filename": filename}));
    }

    fn edit(&mut self, method: &str, view_id: &str, params: serde_json::Value) {
        self.notify("edit",
                    json!({
            "method": method,
            "view_id": view_id,
            "params": params
        }));
    }

    pub fn insert(&mut self, view_id: &str, chars: &str) {
        self.edit("insert", view_id, json!({"chars": chars}));
    }

    pub fn scroll(&mut self, view_id: &str, (beg, end): (usize, usize)) {
        self.edit("scroll", view_id, json!([beg, end]));
    }

    pub fn click(&mut self,
                 view_id: &str,
                 (line, col, modifiers, click_count): (usize, usize, usize, usize)) {
        self.edit("click", view_id, json!([line, col, modifiers, click_count]));
    }

    pub fn drag(&mut self, view_id: &str, (line, col, modifiers): (usize, usize, usize)) {
        self.edit("drag", view_id, json!([line, col, modifiers]));
    }

    pub fn delete_backward(&mut self, view_id: &str) {
        self.edit("delete_backward", view_id, json!({}));
    }
    pub fn insert_newline(&mut self, view_id: &str) {
        self.edit("insert_newline", view_id, json!({}));
    }
    pub fn move_up(&mut self, view_id: &str) {
        self.edit("move_up", view_id, json!({}));
    }
    pub fn move_up_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_up_and_modify_selection", view_id, json!({}));
    }
    pub fn move_down(&mut self, view_id: &str) {
        self.edit("move_down", view_id, json!({}));
    }
    pub fn move_down_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_down_and_modify_selection", view_id, json!({}));
    }
    pub fn move_left(&mut self, view_id: &str) {
        self.edit("move_left", view_id, json!({}));
    }
    pub fn move_left_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_left_and_modify_selection", view_id, json!({}));
    }
    pub fn move_right(&mut self, view_id: &str) {
        self.edit("move_right", view_id, json!({}));
    }
    pub fn move_right_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_right_and_modify_selection", view_id, json!({}));
    }
    pub fn scroll_page_up(&mut self, view_id: &str) {
        self.edit("scroll_page_up", view_id, json!({}));
    }
    pub fn page_up(&mut self, view_id: &str) {
        self.edit("page_up", view_id, json!({}));
    }
    pub fn page_up_and_modify_selection(&mut self, view_id: &str) {
        self.edit("page_up_and_modify_selection", view_id, json!({}));
    }
    pub fn scroll_page_down(&mut self, view_id: &str) {
        self.edit("scroll_page_down", view_id, json!({}));
    }
    pub fn page_down(&mut self, view_id: &str) {
        self.edit("page_down", view_id, json!({}));
    }
    pub fn page_down_and_modify_selection(&mut self, view_id: &str) {
        self.edit("page_down_and_modify_selection", view_id, json!({}));
    }
}
