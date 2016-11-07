use std::io::{Read,Write};
use std::sync::mpsc;
use std::thread;
use std::fs::{OpenOptions};

use std::time::Duration;

use ::{sync, OUTF};

use std::collections::HashMap;

pub fn main() {
    let mut senders = HashMap::new();
    let mut id = 0;

    loop {
        let mut inf = OpenOptions::new().read(true).write(true).create(true).open("INPUT").unwrap();

        let outf = OUTF.lock().unwrap();
        let mut outf = outf.borrow_mut();

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
                        _       => { writeln!(outf, "Error: function not found").unwrap(); }
                    };
                    writeln!(outf, "ID: {}", id).unwrap();
                    id += 1;
                } else { writeln!(outf, "Use sync <name>").unwrap(); }
            } else if input.starts_with("kill") {
                if let Some(input) = input.split_whitespace().collect::<Vec<_>>().get(1) {
                    if let Ok(id) = input.parse::<u32>() {
                        match senders.clone().get(&id) {
                            Some(sender) => { let _ = sender.send(()); senders.remove(&id); writeln!(outf, "Killed {}", id).unwrap(); },
                            None => { writeln!(outf ,"Error: No such id").unwrap(); }
                        }
                    }
                }
            }
        }

        inf.set_len(0).unwrap();
        thread::sleep(Duration::new(15,0));
    }
}
