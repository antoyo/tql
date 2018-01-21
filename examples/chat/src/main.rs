/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

#![feature(proc_macro)]

extern crate chrono;
extern crate handlebars_iron as hbs;
extern crate iron;
extern crate persistent;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tql;
#[macro_use]
extern crate tql_macros;
extern crate urlencoded;

use std::collections::BTreeMap;

use chrono::DateTime;
use chrono::offset::Utc;
use hbs::{DirectorySource, HandlebarsEngine, Template};
use iron::{Iron, IronResult, Plugin, status};
use iron::middleware::Chain;
use iron::method::Method;
use iron::modifier::Set;
use iron::modifiers::Redirect;
use iron::request::Request;
use iron::response::Response;
use iron::typemap::Key;
use r2d2::Pool;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};
use tql::PrimaryKey;
use tql_macros::sql;
use urlencoded::UrlEncodedBody;

struct AppDb;

impl Key for AppDb {
    type Value = Pool<PostgresConnectionManager>;
}

// A Message is a table containing a username, a text and an added date.
#[derive(SqlTable, Serialize)]
struct Message {
    #[serde(skip)]
    id: PrimaryKey,
    username: String,
    message: String,
    #[serde(skip)]
    date_added: DateTime<Utc>,
}

fn chat(req: &mut Request) -> IronResult<Response> {
    let pool = req.get::<persistent::Read<AppDb>>().unwrap();
    let connection = pool.get().unwrap();
    let mut resp = Response::new();

    let mut data = BTreeMap::new();
    if req.method == Method::Post {
        {
            let params = req.get_ref::<UrlEncodedBody>();
            if let Ok(params) = params {
                let username: String = params["username"][0].clone();
                let message: String = params["message"][0].clone();

                // Insert a new message.
                let _ = sql!(Message.insert(
                            username = username,
                            message = message,
                            date_added = Utc::now()
                        ));
            }
        }

        Ok(Response::with((status::Found, Redirect(req.url.clone()))))
    }
    else {
        // Get the last 10 messages by date.
        let messages: Vec<Message> = sql!(Message.sort(-date_added)[..10])
            .expect("get messages");

        data.insert("messages".to_owned(), messages);

        resp.set_mut(Template::new("chat", data))
            .set_mut(status::Ok);
        Ok(resp)
    }
}

fn main() {
    let pool = get_connection_pool();

    {
        let connection = pool.get().unwrap();

        // Create the Message table.
        let _ = sql!(Message.create());
    }

    let mut chain = Chain::new(chat);
    chain.link(persistent::Read::<AppDb>::both(pool));
    let mut handlebars = HandlebarsEngine::new();
    handlebars.add(Box::new(DirectorySource::new("./templates/", ".hbs")));
    // TODO: maybe load?
    if let Err(error) = handlebars.reload() {
        panic!("{}", error);
    }
    chain.link_after(handlebars);
    println!("Running on http://localhost:3000");
    Iron::new(chain).http("localhost:3000").unwrap();
}

fn get_connection_pool() -> Pool<PostgresConnectionManager> {
    let manager = r2d2_postgres::PostgresConnectionManager::new("postgres://test:test@localhost/database", TlsMode::None).unwrap();
    r2d2::Pool::new(manager).unwrap()
}
