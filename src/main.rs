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
    name: String,
    age: i32,
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
    println!("{}, {}", person.name, person.age);
}

fn show_person_with_address(person: Person) {
    println!("{}, {}", person.name, person.age);
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

    let _ = sql!(Address.create());
    let _ = sql!(Person.create());
    let _ = sql!(Address.insert(number = 42, street = "Street Ave"));
    let address = sql!(Address.get(1)).unwrap();
    let _ = sql!(Person.insert(name = "value1", age = 42, address = address));
    let _ = sql!(Person.insert(name = "value2", age = 24, address = address));
    let _ = sql!(Person.insert(name = "value3", age = 12, address = address));

    println!(to_sql!(Person.filter(name == "value1")));
    let people = sql!(Person.filter(name == "value1"));
    show_people(people);
    let people = sql!(Person.filter(name == "value1" && age < 100).sort(-age));
    show_people(people);
    //sql!(Person.filter(name == "value1" && age < 100u32).sort(-age));
    //sql!(Person.filter(name == "value1" && age < 100u64).sort(-age));
    //sql!(Person.filter(name == "value1" && age < 100i8).sort(-age));
    let people = sql!(Person.filter(age < 100 && name == "value1").sort(-age));
    show_people(people);
    let people = sql!(Person.filter(age >= 42).sort(age));
    show_people(people);
    let people = sql!(Person.filter(age >= 42 || name == "te'\"\\st"));
    show_people(people);
    //let people = sql!(Person.filter(age >= b'f' || name == 't'));
    //let people = sql!(Person.filter(age >= b"test"));
    //let people = sql!(Person.filter(age >= r#""test""#));
    //let people = sql!(Person.filter(age >= 3.141592f32));
    //let people = sql!(Person.filter(age >= 3.141592f64));
    //let people = sql!(Person.filter(age >= 3.141592));
    //let people = sql!(Person.filter(age >= 42).sort(nam));
    //let people = sql!(Person.filter(ag >= 42));
    //let people = sql!(Person.filter(age == true));
    //let people = sql!(Person.filter(age == false));
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
    //sql!(Person.filter(age == 42)[index]);
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
    //println!(to_sql!(Person.filter(age < 100 && name == "value1").sort(*age, *name)));
    //println!("{}", to_sql!(Prson.filter(name == "value")));
    //println!("{}", to_sql!(TestTable.flter(name == "value")));

    let people = sql!(Person.all());
    show_people(people);

    let values = ["value1", "value2", "value3"];
    for &value in &values {
        println!("Filtre: {}", value);
        let people = sql!(Person.filter(name == value));
        show_people(people);
    }

    let value = 20;
    println!("Filtre: age > {}", value);
    let people = sql!(Person.filter(age > value));
    show_people(people);

    let value1 = "value1";
    println!("Filtre: age > {} && name == {}", value, value1);
    let people = sql!(Person.filter(age > value && name == value1));
    show_people_with_address(people);

    //let value1 = 42;
    //println!("Filtre: age > {} && name == {}", value, value1);
    //let people = sql!(Person.filter(age > value && name == value1));
    //show_people(people);

    //let value = 20i64;
    //println!("Filtre: age > {}", value);
    //let people = sql!(Person.filter(age > value));
    //show_people(people);

    let people = sql!(Person.all().join(address));
    show_people_with_address(people);

    let people = sql!(Person.filter(address == address));
    show_people(people);

    let person = sql!(Person.get(1));
    show_person_option(person);

    let person = sql!(Person.get(2));
    show_person_option(person);

    let index = 3i32;
    let person = sql!(Person.get(index));
    show_person_option(person);

    let person = sql!(Person.get(age == 24));
    show_person_option(person);

    let person = sql!(Person.get(age == 24 && name == "value2"));
    show_person_option(person);

    let person = sql!(Person.get(age == 24 && name == "value3"));
    show_person_option(person);

    let people = sql!(Person.filter(age > 10).sort(age)[1..3]);
    show_people(people);

    let people = sql!(Person.filter((age < 100 && name == "value1")));
    show_people(people);

    println!(to_sql!(Person.filter(!(age < 100 && name == "value1"))));
    let people = sql!(Person.filter(!(age < 100 && name == "value1")));
    show_people(people);

    println!(to_sql!(Person.filter(!(age < 100))));
    let people = sql!(Person.filter(!(age < 100)));
    show_people(people);

    println!(to_sql!(Person.filter(name == "value2" || age < 100 && name == "value1")));
    let people = sql!(Person.filter(name == "value2" || age < 100 && name == "value1"));
    show_people(people);

    println!(to_sql!(Person.filter((name == "value2" || age < 100) && name == "value1")));
    let people = sql!(Person.filter((name == "value2" || age < 100) && name == "value1"));
    show_people(people);

    let num_updated = match sql!(Person.get(1).update(name = "value1", age = 55)) {
        Ok(number) => number,
        Err(error) => {
            println!("Error: {}", error);
            0
        },
    };
    println!("{} updated entries", num_updated);

    let people = sql!(Person.filter((name == "value2" || age < 100) && name == "value1"));
    show_people(people);

    let new_age = 42i32;
    let _ = sql!(Person.filter(id == 1).update(name = "value1", age = new_age));

    let num_inserted = match sql!(Person.insert(name = "Me", age = 91, address = address)) {
        Ok(number) => number,
        Err(error) => {
            println!("Error: {}", error);
            0
        },
    };
    println!("{} inserted entries", num_inserted);

    let people = sql!(Person.all());
    show_people(people);

    //to_sql!();
    //to_sql!(Person);
    //to_sql!(Person());

    //println!("{}", b"\u{a66e}"); // TODO: faire cette même vérification dans tql.

    println!(to_sql!(Person.filter(name == "Me").delete()));
    let num_deleted = match sql!(Person.filter(name == "Me").delete()) {
        Ok(number) => number,
        Err(error) => {
            println!("Error: {}", error);
            0
        },
    };
    println!("{} deleted entries", num_deleted);

    let people = sql!(Person.all());
    show_people(people);

    //let age = 42;
    //let _ = sql!(Person.insert(name = 42, age = 91));
    //let _ = sql!(Person.insert(name = age, age = 91));
    //let _ = sql!(Person.filter(name == 42).delete());
    //let _ = sql!(Person.filter(name == age).delete());
    //let _ = sql!(Person.filter(id == 1).update(name = 42, age = new_age));
    //let _ = sql!(Person.filter(id == 1).update(name = age, age = new_age));

    //let _ = sql!(Person.all(id == 1));
    //let _ = sql!(Person.filter(id == 1).delete(id == 1));

    //let _ = to_sql!(Person.all().join(name, age));
    //let _ = to_sql!(Person.all().join(address, address)); // TODO: devrait causer une erreur.

    let _ = sql!(Person.drop());
    let _ = sql!(Address.drop());
}
