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

extern crate chrono;
extern crate postgres;
#[macro_use]
extern crate tql;
#[macro_use]
extern crate tql_macros;

use std::env;

use chrono::DateTime;
use chrono::offset::Utc;
use postgres::{Connection, TlsMode};
use tql::PrimaryKey;

// A TodoItem is a table containing a text, an added date and a done boolean.
#[derive(SqlTable)]
struct TodoItem {
    id: PrimaryKey,
    text: String,
    date_added: DateTime<Utc>,
    done: bool,
}

fn add_todo_item(connection: Connection, text: String) {
    // Insert the new item.
    let result = sql!(TodoItem.insert(text = text, date_added = Utc::now(), done = false));
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

fn do_todo_item(cx: Connection, id: i32) {
    // Update the item to make it done.
    let result = sql!(cx, TodoItem.get(id).update(done = true));
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

fn list_todo_items(connection: &Connection, show_done: bool) -> Result<(), ::postgres::Error> {
    let items =
        if show_done {
            // Show the last 10 todo items.
            sql!(TodoItem.sort(-date_added)[..10])?
        }
        else {
            // Show the last 10 todo items that are not done.
            sql!(TodoItem.filter(done == false).sort(-date_added)[..10])?
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

    Ok(())
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
            list_todo_items(&connection, show_done)
                .expect("Cannot fetch todo items");
        },
        command => println!("Unknown command {}", command),
    }
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", TlsMode::None).unwrap()
}
