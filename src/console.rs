use std::io::{self,Write};
use std::sync::mpsc;
use std::thread;

use ::sync;

use std::collections::HashMap;

pub fn main() {
    let mut senders = HashMap::new();
    let mut id = 0;

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if let Ok(_) = io::stdin().read_line(&mut input) {
            let input = input.trim();
            if input.starts_with("sync") {
                if let Some(func) = input.split_whitespace().collect::<Vec<_>>().get(1) {
                    let (sendr, recvr) = mpsc::channel::<()>();
                    senders.insert(id, sendr);
                    match *func {
                        "derpy" => { thread::spawn(move || sync::derpy::main(&recvr)); },
                        "e621"  => { thread::spawn(move || sync::e621::main(&recvr)); },
                        "dan"   => { thread::spawn(move || sync::danbooru::main(&recvr)); }
                        _       => println!("Error: function not found")
                    };
                    println!("ID: {}", id);
                    id += 1;
                } else { println!("Use sync <name>") }
            } else if input.starts_with("kill") {
                if let Some(input) = input.split_whitespace().collect::<Vec<_>>().get(1) {
                    if let Ok(id) = input.parse::<u32>() {
                        match senders.clone().get(&id) {
                            Some(sender) => { let _ = sender.send(()); senders.remove(&id); println!("Killed {}", id); },
                            None => println!("Error: No such id")
                        }
                    }
                } else {
                    println!("Use kill <id>");
                }
            }
        }
    }
}
