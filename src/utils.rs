extern crate image;

use self::image::FilterType;

use std::fmt::Display;
use std::io::Write;
use std::fs::{File,create_dir,read_dir};
use std::path::Path;

use ::OUTF;

/// Написать в `OUTPUT`
pub fn log<T: Display>(s: T) {
    let outf = OUTF.lock().unwrap(); 
    let mut outf = outf.borrow_mut();
    writeln!(outf, "{}", s).unwrap();
}

/// Сохраняет картинку & создаёт к ней превью
pub fn save_image(dir: &Path, name: &str, file: &[u8]) {
    if read_dir("assets/images").is_err() { create_dir("assets/images").unwrap(); }
    if read_dir("assets/images/preview").is_err() { create_dir("assets/images/preview").unwrap(); }

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
pub fn arr_eq<T: PartialEq>(first: &mut Vec<T>, second: &mut Vec<T>) -> bool {
    first.dedup();
    second.dedup();
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
