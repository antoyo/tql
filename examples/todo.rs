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

#![feature(box_patterns, plugin)]
#![plugin(tql_macros)]

extern crate chrono;
extern crate postgres;
extern crate tql;

use std::env;

use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use postgres::{Connection, SslMode};
use tql::PrimaryKey;

// A TodoItem is a table containing a text, an added date and a done boolean.
#[SqlTable]
struct TodoItem {
    id: PrimaryKey,
    text: String,
    date_added: DateTime<UTC>,
    done: bool,
}

fn add_todo_item(connection: Connection, text: String) {
    // Insert the new item.
    let result = sql!(TodoItem.insert(text = text, date_added = UTC::now(), done = false));
    if let Err(err) = result {
        println!("Failed to add the item ({})", err);
    }
    else {
        println!("Item added");
    }
}

fn delete_todo_item(connection: Connection, id: i32) {
    // Delete the item.
    let result = sql!(TodoItem.get(id).delete());
    if let Err(err) = result {
        println!("Failed to delete the item ({})", err);
    }
    else {
        println!("Item deleted");
    }
}

fn do_todo_item(connection: Connection, id: i32) {
    // Update the item to make it done.
    let result = sql!(TodoItem.get(id).update(done = true));
    if let Err(err) = result {
        println!("Failed to do the item ({})", err);
    }
    else {
        println!("Item done");
    }
}

fn get_id(args: &mut env::Args) -> Option<i32> {
    if let Some(arg) = args.next() {
        if let Ok(id) = arg.parse() {
            return Some(id);
        }
        else {
            println!("Please provide a valid id");
        }
    }
    else {
        println!("Missing argument: id");
    }
    None
}

fn list_todo_items(connection: Connection, show_done: bool) {
    let items =
        if show_done {
            // Show the last 10 todo items.
            sql!(TodoItem.sort(-date_added)[..10])
        }
        else {
            // Show the last 10 todo items that are not done.
            sql!(TodoItem.filter(done == false).sort(-date_added)[..10])
        };

    for item in items {
        let done_text =
            if item.done {
                "(✓)"
            }
            else {
                ""
            };
        println!("{}. {} {}", item.id, item.text, done_text);
    }
}

fn main() {
    let connection = get_connection();

    // Create the table.
    let _ = sql!(TodoItem.create());

    let mut args = env::args();
    args.next();

    let command = args.next().unwrap_or("list".to_owned());
    match command.as_ref() {
        "add" => {
            if let Some(item_text) = args.next() {
                add_todo_item(connection, item_text);
            }
            else {
                println!("Missing argument: task");
            }
        },
        "delete" => {
            if let Some(id) = get_id(&mut args) {
                delete_todo_item(connection, id);
            }
        },
        "do" => {
            if let Some(id) = get_id(&mut args) {
                do_todo_item(connection, id);
            }
        },
        "list" => {
            let show_done = args.next() == Some("--show-done".to_owned());
            list_todo_items(connection, show_done);
        },
        command => println!("Unknown command {}", command),
    }
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}
