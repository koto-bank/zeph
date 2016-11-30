extern crate image;

use self::image::FilterType;

use std::fmt::Display;
use std::io::{Write,Read};
use std::fs::{File,create_dir,read_dir};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::collections::HashMap;
use std::sync::Mutex;
use std::cell::RefCell;

use super::{Table,Parser};
use ::{LOG,CONFIG};
use ::sync;


lazy_static!{
    static ref SENDERS : Mutex<RefCell<HashMap<u32, mpsc::Sender<()>>>> = Mutex::new(RefCell::new(HashMap::new()));
    static ref ID : Mutex<RefCell<u32>> = Mutex::new(RefCell::new(0));
}

pub fn exec_command(input: &str) {
    let senders = SENDERS.lock().unwrap();
    let id = ID.lock().unwrap();
    let mut senders = senders.borrow_mut();
    let mut id = id.borrow_mut();

    let input = input.trim();
    if input.starts_with("sync") {
        if let Some(func) = input.split_whitespace().collect::<Vec<_>>().get(1) {
            let (sendr, recvr) = mpsc::channel::<()>();
            senders.insert(*id, sendr);
            match *func {
                "derpy" => { thread::spawn(move || sync::derpy::main(&recvr)); },
                "e621"  => { thread::spawn(move || sync::e621::main(&recvr)); },
                "dan"   => { thread::spawn(move || sync::danbooru::main(&recvr)); }
                "kona"  => { thread::spawn(move || sync::konachan::main(&recvr)); }
                "gel"   => { thread::spawn(move || sync::gelbooru::main(&recvr)); }
                _       => { log("Error: function not found") }
            };
            log(format!("ID: {}", *id));
            *id += 1;
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

/// Log something
pub fn log<T: Display>(s: T) {
    let log = LOG.lock().unwrap();
    log.borrow_mut().push(format!("{}", s));
}

/// Save image & generate preview
pub fn save_image(dir: &Path, name: &str, file: &[u8]) {
    if read_dir(config!("images-directory")).is_err() { create_dir(config!("images-directory")).unwrap(); }
    if read_dir(format!("{}/preview", config!("images-directory"))).is_err() { create_dir(format!("{}/preview", config!("images-directory"))).unwrap(); }

    let prev = match image::load_from_memory(file) {
        Ok(x) => x.resize(500, 500, FilterType::Nearest),
        Err(x)  => {
            log(x);
            return
        }
    };

    let mut f = File::create(dir.join(name)).unwrap();
    let mut prevf = File::create(dir.join("preview").join(name)).unwrap();

    f.write(file).unwrap();
    prev.save(&mut prevf, image::JPEG).unwrap();
}

/// Are arrays equeal?
pub fn arr_eq<T: Ord + PartialEq>(first: &mut Vec<T>, second: &mut Vec<T>) -> bool {
    first.sort();
    second.sort();
    first == second
}


/// Second includes first?
pub fn includes<T: PartialEq>(first: &[T], second: &[T]) -> bool {
    let r = first.len();
    let mut c = 0;
    for f in first {
        if second.iter().any(|x| x == f) {
            c += 1;
        }
    }

    r == c
}

pub fn open_config() -> Table {
    let mut file = match File::open("Config.toml") {
        Ok(x)   => x,
        Err(_)  => panic!("No config file")
    };
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    Parser::new(&s).parse().unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn arr_eq_test() {
        assert!(arr_eq(&mut vec!["first".to_string(), "second".to_string()],&mut vec!["second".to_string(), "first".to_string()]));
    }

    #[test]
    fn arr_incl_test() {
        assert!(includes(&vec!["a","b"], &vec!["a", "b", "c"]));
    }
}
