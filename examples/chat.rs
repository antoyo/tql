/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate chrono;
extern crate handlebars_iron as hbs;
extern crate iron;
extern crate persistent;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rustc_serialize;
extern crate tql;
extern crate urlencoded;

use std::collections::BTreeMap;

use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use hbs::{HandlebarsEngine, Template};
use iron::{Iron, IronResult, Plugin, status};
use iron::middleware::Chain;
use iron::method::Method;
use iron::modifier::Set;
use iron::modifiers::Redirect;
use iron::request::Request;
use iron::response::Response;
use iron::typemap::Key;
use postgres::SslMode;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use rustc_serialize::json::{Json, ToJson};
use tql::PrimaryKey;
use urlencoded::UrlEncodedBody;

struct AppDb;

impl Key for AppDb {
    type Value = Pool<PostgresConnectionManager>;
}

// A Message is a table containing a username, a text and an added date.
#[SqlTable]
struct Message {
    id: PrimaryKey,
    username: String,
    message: String,
    date_added: DateTime<UTC>,
}

impl ToJson for Message {
    fn to_json(&self) -> Json {
        let mut map = BTreeMap::new();
        map.insert("username".to_owned(), self.username.to_json());
        map.insert("message".to_owned(), self.message.to_json());
        map.to_json()
    }
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
                            date_added = UTC::now()
                        ));
            }
        }

        Ok(Response::with((status::Found, Redirect(req.url.clone()))))
    }
    else {
        // Get the last 10 messages by date.
        let messages: Vec<Message> = sql!(Message.sort(-date_added)[..10]);

        data.insert("messages".to_owned(), messages.to_json());

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
    chain.link_after(HandlebarsEngine::new("./examples/templates/", ".hbs"));
    println!("Running on http://localhost:3000");
    Iron::new(chain).http("localhost:3000").unwrap();
}

fn get_connection_pool() -> Pool<PostgresConnectionManager> {
    let manager = r2d2_postgres::PostgresConnectionManager::new("postgres://test:test@localhost/database", SslMode::None).unwrap();
    let config = r2d2::Config::builder().pool_size(1).build();
    r2d2::Pool::new(config, manager).unwrap()
}
