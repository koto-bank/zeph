#[macro_use] extern crate nickel;
#[macro_use] extern crate lazy_static;

extern crate rustc_serialize;
extern crate multipart;
extern crate nickel_jwt_session;
extern crate time;

use nickel::{Nickel,
            Request,
            Response,
            MiddlewareResult,
            HttpRouter,
            StaticFilesHandler,
            QueryString,
            MediaType,
            FormBody};
use nickel::status::StatusCode;
use nickel::extensions::Redirect;

use nickel_jwt_session::*;

use std::fs::{File,OpenOptions,remove_file};
use std::path::Path;
use std::io::Read;
use std::thread;
use std::collections::HashMap;

use multipart::server::{Multipart, SaveResult};

use rustc_serialize::json;

mod db;
mod sync;
mod commands;
mod utils;

use db::{Db,VoteImageError};
use utils::save_image;

use std::sync::Mutex;
use std::cell::RefCell;

lazy_static! {
    pub static ref DB : Mutex<Db> = Mutex::new(Db::new());

    /// Использование лежит в utils
    pub static ref OUTF : Mutex<RefCell<File>> = Mutex::new(RefCell::new(OpenOptions::new().append(true).create(true).open("OUTPUT").unwrap()));
}


macro_rules! routes(
    { $serv:ident, $($method:ident $($path:expr),+ => $fun:ident),+ } => {
        {
            $($(
                    $serv.$method($path, $fun);
               )+)+
        }
     };
);

/*fn index_n_search<'a, D>(_request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    response.render("src/templates/index.html", &[0]) // Вот тут и ниже так надо, чтобы не пересобирать программу при изменении HTML
}*/

/*fn show<'a, D>(request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let mut data = HashMap::new();
    let id = request.param("id").unwrap().parse::<i32>().unwrap();
    let cont = DB.lock().unwrap().get_image(id).unwrap();
    data.insert("image", cont);
    response.render("src/templates/show.html", &data)
}*/

fn upload_image<'mw>(req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    if let Some(username) = req.authorized_user() {
        if let Ok(mut multipart) = Multipart::from_request(req) {
            match multipart.save_all() {
                SaveResult::Full(entries) | SaveResult::Partial(entries, _)  => {
                    if let Some(savedfile) = entries.files.get("image") {
                        if let Some(ref filename) = savedfile.filename {
                            if let Some(tags) = entries.fields.get("tags") {
                                let tags = tags.split_whitespace().map(String::from).collect::<Vec<_>>();
                                let mut body = Vec::new();
                                let _ = File::open(&savedfile.path).unwrap().read_to_end(&mut body);
                                let name = DB.lock().unwrap().add_with_tags_name(&tags, filename.split('.').collect::<Vec<_>>()[1], &username).unwrap();

                                save_image(Path::new("assets/images"), &name, &body);

                                res.redirect("/")

                            } else { res.send("No tags found") }
                        } else { res.send("Can't get filename") }
                    } else { res.send("Can't load file") }
                },

                SaveResult::Error(e) =>  res.send(format!("Server could not handle multipart POST! {:?}", e))
            }
        } else {
            res.set(nickel::status::StatusCode::BadRequest);
            res.send("Not a multipart request")
        }
    } else {
        res.error(StatusCode::Forbidden, "Not logged in")
    }
}

/*fn more<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let offset = request.query().get("offset").unwrap().parse::<usize>().unwrap();

    let images = match request.query().get("q") {
        Some(x) =>  DB.lock().unwrap().by_tags(25, offset, &x.to_lowercase().split_whitespace().map(String::from).collect::<Vec<_>>()).unwrap(),
        None    =>  DB.lock().unwrap().get_images(25, offset).unwrap()
    };

    response.set(MediaType::Json);
    response.send(json::encode(&images).unwrap())
}*/

fn adduser<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let body = try_with!(response, request.form_body());
    if let (Some(login), Some(pass), Some(confirm_pass)) = (body.get("login"), body.get("password"),body.get("confirm_password")) {
        if pass == confirm_pass {
            if !pass.trim().is_empty() && !login.trim().is_empty() {
                if let Ok(res) = DB.lock().unwrap().add_user(login,pass) {
                    if res {
                        response.set_jwt_user(login);
                        response.redirect("/")
                    } else {
                        response.send("User already exists")
                    }
                } else {
                    response.error(StatusCode::InternalServerError, "Internal server error")
                }
            } else {
                response.send("Empty login/pass")
            }
        } else {
            response.send("Password and confirmation are not equeal")
        }
    } else {
        response.send("No data")
    }
}

fn login<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let body = try_with!(response, request.form_body());
    if let (Some(login), Some(pass)) = (body.get("login"), body.get("password")) {
        match DB.lock().unwrap().check_user(login, pass) {
            Ok(x) => match x {
                Some(x) => if x {
                    response.set_jwt_user(login);
                    response.redirect("/")
                } else {
                    response.send("Incorrent password")
                },
                _   => response.send("No such user")
            },
            Err(e) =>  panic!(e)
        }
    } else {
        response.send("No login/pass")
    }
}

fn user_status<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {

    #[derive(RustcEncodable)]
    struct UserStatus {
        logined: bool,
        name: Option<String>
    }

    response.set(MediaType::Json);

    let (logined,name) = match request.authorized_user() {
        Some(user)  => (true, Some(user)),
        None        => (false, None)
    };

    response.send(json::encode(&UserStatus{
        logined: logined,
        name: name
    }).unwrap())
}

fn delete<'a, D>(request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let id = request.param("id").unwrap().parse::<i32>().unwrap();
    let name = DB.lock().unwrap().delete_image(id).unwrap();
    remove_file(format!("assets/images/{}", name)).unwrap();
    remove_file(format!("assets/images/preview/{}", name)).unwrap();
    response.redirect("/")
}

fn vote_image<'a, D>(request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let query = request.query() as *const nickel::Query;
    let (id,vote) = unsafe { // Бяка, конечно, но query() почему-то берёт mutable reference и не даёт использовать другим
        let q = &*query;
        (q.get("id"),q.get("vote"))
    };

    if let (Some(id), Some(vote)) = (id,vote) {
        if let Some(name) = request.authorized_user() {
            if let (Ok(vote),Ok(id)) = (vote.parse::<bool>(),id.parse::<i32>()) {
                match DB.lock().unwrap().vote_image(&name, id, vote).unwrap() {
                    Ok(newv)                        => response.send(newv.to_string()),
                    Err(VoteImageError::Already)    => response.send("Already voted that"),
                    Err(VoteImageError::NoImage)    => response.send("No such image")
                }
            } else {
                response.send("Invalid data")
            }
        } else {
            response.send("Not logged in")
        }
    } else {
        response.send("No data")
    }
}

fn main() {
    let mut server = Nickel::new();

    server.utilize(StaticFilesHandler::new("assets"));
    server.utilize(SessionMiddleware::new(&time::now().to_timespec().sec.to_string()));

    routes!{server,
        get "/","/search" => index_n_search,
        get "/show/:id" => show,
        get "/more" => more,
        get "/user_status" => user_status,
        get "/delete/:id" => delete,
        get "/vote_image" => vote_image,

        post "/upload_image" => upload_image,
        post "/login" => login,
        post "/adduser" => adduser
    };

    thread::spawn(commands::main);

    let _server = server.listen("127.0.0.1:3000");
}
