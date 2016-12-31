//! Routes that help to work w/ users

use {DB,json};

use iron::prelude::*;
use iron::status;
use iron::modifiers::RedirectRaw as Redirect;
use iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};

use urlencoded::UrlEncodedBody;
use session::SessionRequestExt;

use Login;

pub fn user_status(req: &mut Request) -> IronResult<Response> {
    #[derive(RustcEncodable)]
    struct UserStatus {
        logined: bool,
        name: Option<String>
    }

    let (logined,name) = match req.session().get::<Login>()? {
        Some(user)  => (true, Some(user.0)),
        None        => (false, None)
    };

    let mut response = Response::new();

    response
        .set_mut(Mime(TopLevel::Application, SubLevel::Json,
                      vec![(Attr::Charset, Value::Utf8)]))
        .set_mut(json::encode(&UserStatus{logined: logined,name: name}).unwrap())
        .set_mut(status::Ok);
    Ok(response)
}

pub fn login(req: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();

    let body = match req.get::<UrlEncodedBody>() {
        Ok(data) => data,
        Err(_)  => return Ok(Response::with(status::BadRequest))
    };

    if let (Some(login), Some(pass)) = (body.get("login"),body.get("password")) {
        match DB.lock().unwrap().check_user(&login[0], &pass[0]).unwrap() {
            Some(x) if x => {
                req.session().set(Login(login[0].clone()))?;
                response
                    .set_mut(Redirect("/".to_string()))
                    .set_mut(status::Found);
                Ok(response)
            },
            Some(_) => Ok(Response::with((status::BadRequest,"Incorrect login/pass"))),
            None  => Ok(Response::with((status::Ok,"No such user")))
        }
    } else {
        Ok(Response::with((status::BadRequest,"No login/pass")))
    }
}

pub fn adduser(req: &mut Request) -> IronResult<Response> {

    let body = match req.get::<UrlEncodedBody>() {
        Ok(data) => data,
        Err(_)  => return Ok(Response::with(status::BadRequest))
    };

    Ok(if let (Some(login), Some(pass), Some(confirm_pass)) = (body.get("login"), body.get("password"),body.get("confirm_password")) {
        let (login,pass,confirm_pass) = (&login[0], &pass[0], &confirm_pass[0]);
        if pass == confirm_pass {
            if !pass.trim().is_empty() && !login.trim().is_empty() {
                match DB.lock().unwrap().add_user(login,pass) {
                    Ok(res)   => {
                        if res {
                            let mut response = Response::new();
                            req.session().set(Login(login.clone()))?;
                            response
                                .set_mut(Redirect("/".to_string()))
                                .set_mut(status::Found);
                            response
                        } else {
                            Response::with((status::Ok,"User already exists"))
                        }
                    },
                    Err(e)  => Response::with((status::InternalServerError, format!("Internal server error: {}", e)))
                }
            } else {
                Response::with((status::BadRequest,"Empty login/pass"))
            }
        } else {
            Response::with((status::Ok,"Password and confirmation are not equeal"))
        }
    } else {
        Response::with((status::BadRequest,"No data"))
    })
}
