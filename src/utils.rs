extern crate image;

use self::image::FilterType;

use std::fmt::Display;
use std::io::{Write,Read};
use std::fs::{File,create_dir,read_dir};
use std::path::Path;

use super::{Table,Parser};

use ::{OUTF,CONFIG};

/// Написать в `OUTPUT`
pub fn log<T: Display>(s: T) {
    let outf = OUTF.lock().unwrap();
    let mut outf = outf.borrow_mut();
    writeln!(outf, "{}", s).unwrap();
}

/// Сохраняет картинку & создаёт к ней превью
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

/// Равны ли массивы
pub fn arr_eq<T: Ord + PartialEq>(first: &mut Vec<T>, second: &mut Vec<T>) -> bool {
    first.sort();
    second.sort();
    first == second
}


/// Включает ли второй массив первый
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
