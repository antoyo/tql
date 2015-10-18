#![feature(plugin)]
#![plugin(tql_macros)]

#[macro_use]
extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[derive(Debug)]
struct Person {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    address: ForeignKey<Address>,
}

#[SqlTable]
#[derive(Debug)]
struct Address {
    id: PrimaryKey,
    number: i32,
    street: String,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

fn result() -> i64 {
    2
}

struct Strct {
    x: i64,
}

impl Strct {
    fn result(&self) -> i64 {
        self.x
    }
}

fn show_person(person: Person) {
    println!("{}, {}", person.field1, person.field2);
}

fn show_person_with_address(person: Person) {
    println!("{}, {}", person.field1, person.field2);
    match person.address {
        Some(address) => {
            println!("Address: {}, {}", address.number, address.street);
        },
        None => println!("Pas d’adresse"),
    }
}

fn show_person_option(person: Option<Person>) {
    match person {
        Some(person) => {
            println!("One result");
            show_person(person);
        },
        None => println!("No person"),
    }
}

fn show_people(people: Vec<Person>) {
    for person in people {
        show_person(person);
    }
}

fn show_people_with_address(people: Vec<Person>) {
    for person in people {
        show_person_with_address(person);
    }
}

fn main() {
    let connection = get_connection();
    println!(to_sql!(Person.filter(field1 == "value1")));
    let people = sql!(Person.filter(field1 == "value1"));
    show_people(people);
    let people = sql!(Person.filter(field1 == "value1" && field2 < 100).sort(-field2));
    show_people(people);
    //sql!(Person.filter(field1 == "value1" && field2 < 100u32).sort(-field2));
    //sql!(Person.filter(field1 == "value1" && field2 < 100u64).sort(-field2));
    //sql!(Person.filter(field1 == "value1" && field2 < 100i8).sort(-field2));
    let people = sql!(Person.filter(field2 < 100 && field1 == "value1").sort(-field2));
    show_people(people);
    let people = sql!(Person.filter(field2 >= 42).sort(field2));
    show_people(people);
    let people = sql!(Person.filter(field2 >= 42 || field1 == "te'\"\\st"));
    show_people(people);
    //let people = sql!(Person.filter(field2 >= b'f' || field1 == 't'));
    //let people = sql!(Person.filter(field2 >= b"test"));
    //let people = sql!(Person.filter(field2 >= r#""test""#));
    //let people = sql!(Person.filter(field2 >= 3.141592f32));
    //let people = sql!(Person.filter(field2 >= 3.141592f64));
    //let people = sql!(Person.filter(field2 >= 3.141592));
    //let people = sql!(Person.filter(field2 >= 42).sort(field));
    //let people = sql!(Person.filter(field >= 42));
    //let people = sql!(Person.filter(field2 == true));
    //let people = sql!(Person.filter(field2 == false));
    //sql!(Person.all()[.."auinesta"]);
    //sql!(Person.all()[true..false]);
    let people = sql!(Person.all()[..2]);
    show_people(people);
    println!(to_sql!(Person.all()[1..3]));
    let people = sql!(Person.all()[1..3]);
    show_people(people);
    println!(to_sql!(Person.all()[2]));
    let person = sql!(Person.all()[2]);
    show_person_option(person);
    let person = sql!(Person.all()[42]);
    show_person_option(person);
    println!(to_sql!(Person.all()[2 - 1]));
    let person = sql!(Person.all()[2 - 1]);
    show_person_option(person);
    println!(to_sql!(Person.all()[..2 - 1]));
    let people = sql!(Person.all()[..2 - 1]);
    show_people(people);
    println!(to_sql!(Person.all()[2 - 1..]));
    let people = sql!(Person.all()[2 - 1..]);
    show_people(people);
    //let index = 24;
    //sql!(Person[index]);
    //sql!(Person.filter(field2 == 42)[index]);
    let index = 2i64;
    let person = sql!(Person.all()[index]);
    show_person_option(person);
    let index = 1i64;
    let end_index = 3i64;
    println!(to_sql!(Person.all()[index..end_index]));
    let people = sql!(Person.all()[index..end_index]);
    show_people(people);
    println!(to_sql!(Person.all()[result()]));
    let person = sql!(Person.all()[result()]);
    show_person_option(person);
    let strct = Strct{
        x: 2,
    };
    println!(to_sql!(Person.all()[strct.result()]));
    let person = sql!(Person.all()[strct.result()]);
    show_person_option(person);
    println!(to_sql!(Person.all()[index + 1]));
    let person = sql!(Person.all()[index + 1]);
    show_person_option(person);
    let index = -2i64;
    println!(to_sql!(Person.all()[-index]));
    let people = sql!(Person.all()[-index]);
    show_person_option(people);
    println!(to_sql!(Person.all()[-index as i64]));
    let people = sql!(Person.all()[-index as i64]);
    show_person_option(people);
    //println!(to_sql!(Person.filter(field2 < 100 && field1 == "value1").sort(*field2, *field1)));
    //println!("{}", to_sql!(Prson.filter(field1 == "value")));
    //println!("{}", to_sql!(TestTable.flter(field1 == "value")));

    let people = sql!(Person.all());
    show_people(people);

    let values = ["value1", "value2", "value3"];
    for &value in &values {
        println!("Filtre: {}", value);
        let people = sql!(Person.filter(field1 == value));
        show_people(people);
    }

    let value = 20;
    println!("Filtre: field2 > {}", value);
    let people = sql!(Person.filter(field2 > value));
    show_people(people);

    let value1 = "value1";
    println!("Filtre: field2 > {} && field1 == {}", value, value1);
    let people = sql!(Person.filter(field2 > value && field1 == value1));
    show_people_with_address(people);

    //let value1 = 42;
    //println!("Filtre: field2 > {} && field1 == {}", value, value1);
    //let people = sql!(Person.filter(field2 > value && field1 == value1));
    //show_people(people);

    //let value = 20i64;
    //println!("Filtre: field2 > {}", value);
    //let people = sql!(Person.filter(field2 > value));
    //show_people(people);

    let people = sql!(Person.all().join(address));
    show_people_with_address(people);

    // TODO: ceci devrait fonctionner.
    //let address = Address {
        //id: 1,
        //number: 42,
        //street: "Street Ave".to_owned(),
    //};
    //sql!(Person.filter(address == address));

    let person = sql!(Person.get(1));
    show_person_option(person);

    let person = sql!(Person.get(2));
    show_person_option(person);

    let index = 3i32;
    let person = sql!(Person.get(index));
    show_person_option(person);

    let person = sql!(Person.get(field2 == 24));
    show_person_option(person);

    let person = sql!(Person.get(field2 == 24 && field1 == "value2"));
    show_person_option(person);

    let person = sql!(Person.get(field2 == 24 && field1 == "value3"));
    show_person_option(person);
}
