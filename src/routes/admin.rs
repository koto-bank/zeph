//! All the admin panel stuff

use {LOG,CONFIG,json,exec_command};

use iron::prelude::*;
use iron::status;
use iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};

use urlencoded::UrlEncodedBody;
use session::SessionRequestExt;

use Login;

pub fn admin_command(req: &mut Request) -> IronResult<Response> {
    if let (Some(curr_username), Some(admin_username)) = (req.session().get::<Login>()?,config!(? "admin-username")) {
        if curr_username.0.to_lowercase() == admin_username.to_lowercase() {
            let body = match req.get::<UrlEncodedBody>() {
                Ok(data) => data,
                Err(_)  => return Ok(Response::with(status::BadRequest))
            };

            if let Some(comm) = body.get("command") {
                exec_command(&comm[0]);
            }

            Ok(Response::with(status::Ok))
        } else {
            Ok(Response::with((status::Forbidden,"Not an admin")))
        }
    } else {
        Ok(Response::with((status::Forbidden,"Not logged in"))) // .. or admin account is not set
    }
}

pub fn admin(req: &mut Request) -> IronResult<Response> {
    if let (Some(curr_username), Some(admin_username)) = (req.session().get::<Login>()?,config!(? "admin-username")) {
        if curr_username.0.to_lowercase() == admin_username.to_lowercase() {
            let page = html!{
                script src="/assets/js/admin.js" {}

                div#log-block style="width:40%; height:50%; overflow-y: auto; border: 1px solid black;" {
                    @for l in LOG.lock().unwrap().iter() {
                        (l)
                    }
                }
                br /
                form#command-form onsubmit="sendCommand(this); return false;" {
                    input name="comm" nameplaceholder="Command" type="text" /
                    input#send-button value="Send" type="submit" /
                }
            };
            Ok(Response::with((status::Ok,page)))
        } else {
            Ok(Response::with((status::Forbidden,"Not an admin")))
        }
    } else {
        Ok(Response::with((status::Forbidden,"Not logged in"))) // .. or admin account is not set
    }
}

pub fn get_log(req: &mut Request) -> IronResult<Response> {
    if let (Some(curr_username), Some(admin_username)) = (req.session().get::<Login>()?,config!(? "admin-username")) {
        if curr_username.0.to_lowercase() == admin_username.to_lowercase() {
            let mut response = Response::new();
            response
                .set_mut(Mime(TopLevel::Application, SubLevel::Json,
                              vec![(Attr::Charset, Value::Utf8)]))
                .set_mut(json::encode(&*LOG.lock().unwrap()).unwrap())
                .set_mut(status::Ok);

            Ok(response)
        } else {
            Ok(Response::with((status::Forbidden,"Not an admin")))
        }
    } else {
        Ok(Response::with((status::Forbidden,"Not logged in"))) // .. or admin account is not set
    }
}
