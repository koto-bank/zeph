use std::io::Read;
use std::sync::mpsc;
use std::thread;
use std::fs::{OpenOptions};

use std::time::Duration;

use ::sync::{self,log};

use std::collections::HashMap;

pub fn main() {
    let mut senders = HashMap::new();
    let mut id = 0;

    loop {
        let mut inf = OpenOptions::new().read(true).write(true).create(true).open("INPUT").unwrap();

        let mut inputs = String::new();
        inf.read_to_string(&mut inputs).unwrap();
        let inputs = inputs.split('\n').collect::<Vec<_>>();

        for input in inputs {
            let input = input.trim();
            if input.starts_with("sync") {
                if let Some(func) = input.split_whitespace().collect::<Vec<_>>().get(1) {
                    let (sendr, recvr) = mpsc::channel::<()>();
                    senders.insert(id, sendr);
                    match *func {
                        "derpy" => { thread::spawn(move || sync::derpy::main(&recvr)); },
                        "e621"  => { thread::spawn(move || sync::e621::main(&recvr)); },
                        "dan"   => { thread::spawn(move || sync::danbooru::main(&recvr)); }
                        "kona"  => { thread::spawn(move || sync::konachan::main(&recvr)); }
                        _       => { log("Error: function not found") }
                    };
                    log(format!("ID: {}", id));
                    id += 1;
                } else { log("Use sync <name>") }
            } else if input.starts_with("kill") {
                if let Some(input) = input.split_whitespace().collect::<Vec<_>>().get(1) {
                    if let Ok(id) = input.parse::<u32>() {
                        match senders.clone().get(&id) {
                            Some(sender) => { let _ = sender.send(()); senders.remove(&id); log(format!("Killed {}", id)) },
                            None => { log("Error: No such id") }
                        }
                    }
                }
            }
        }

        inf.set_len(0).unwrap();
        thread::sleep(Duration::new(15,0));
    }
}
