use clap::{Parser, Subcommand};
use rusqlite::{params, Connection};
use rusqlite::types::Null;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use time::{PrimitiveDateTime, OffsetDateTime, Time, Duration};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Add { item: String },
    Complete { id: i32 },
    Remove { id: i32 },
}

fn add_item(item: String, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    println!("Do you want to add a due date? (y/n)");
    let mut str = String::new();
    std::io::stdin().read_line(&mut str)?;
    if str.chars().nth(0) == Some('y'){
        println!("In how many days do you plan to finish this?");
        let days_str = read_line();
        let days = days_str.trim().parse::<i32>()?;
        println!("At what hour?");
        let hours_str = read_line();
        let hours = hours_str.trim().parse::<i32>()?;
        println!("At what minute?");
        let minutes_str = read_line();
        let minutes = minutes_str.trim().parse::<i32>()?;
        let date_time = PrimitiveDateTime::new(OffsetDateTime::now_local()?.date() + Duration::days(days as i64), Time::from_hms(hours as u8, minutes as u8, 0)?);
        conn.execute("INSERT INTO items (ITEM, DUE_DATE, IS_COMPLETED) VALUES (?, ?, ?)", params![item, date_time, false])?;
    } else {
        conn.execute("INSERT INTO items (ITEM, DUE_DATE, IS_COMPLETED) VALUES (?, ?, ?)", params![item, Null, false])?;
    }
    Ok(())
}

fn read_line() -> String {
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    buf
}

fn list_items(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT * FROM items")?;
    let rows = stmt.query_map((), |rows| Ok((rows.get::<_, i32>(0)?, rows.get::<_, String>(1)?, rows.get::<_, PrimitiveDateTime>(2)?, rows.get::<_, bool>(3)?)))?;
    let mut colored_red_stdout = StandardStream::stdout(ColorChoice::Always);
    let mut colored_yellow_stdout = StandardStream::stdout(ColorChoice::Always);
    colored_red_stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    colored_yellow_stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    println!("{0: <10} | {1: <50} | {2: <20} | {3: <10}", "ID", "Description", "Due Date", "Status");
    let row_vec = rows.map(|rows| rows.unwrap()).collect::<Vec<_>>();
    let now = PrimitiveDateTime::new(OffsetDateTime::now_local()?.date(), OffsetDateTime::now_local()?.time());
    for row in row_vec.iter() {
        let row = row;
        let id = row.0;
        let description = &row.1;
        let due_date = row.2;
        let is_completed = row.3;
        if !is_completed {
            if now < due_date && due_date < now + Duration::days(3) {
                writeln!(colored_yellow_stdout, "{0: <10} | {1: <50} | {2: <20} | {3: <10}", id, description, due_date, is_completed)?;
            } else if PrimitiveDateTime::new(OffsetDateTime::now_local()?.date(), OffsetDateTime::now_local()?.time()) > due_date {
                writeln!(colored_red_stdout, "{0: <10} | {1: <50} | {2: <20} | {3: <10}", id, description, due_date, is_completed)?;
            } else {
                println!("{0: <10} | {1: <50} | {2: <20} | {3: <10}", id, description, due_date, is_completed);
            }
        }
    };
    let overdue_items = &row_vec.iter().filter(|row| row.2 < now).map(|row| row.0).collect::<Vec<_>>()[..];
    let close_items = &row_vec.iter().filter(|row| now + Duration::days(3) > row.2 && row.2 > now).map(|row| row.0).collect::<Vec<_>>()[..];
    if overdue_items.len() > 0 { writeln!(colored_red_stdout, "Items {:?} are overdue! Finish them quickly!", overdue_items)?; }
    if close_items.len() > 0 { writeln!(colored_yellow_stdout, "Items {:?} are close to deadline, you should finish them soon!", close_items)?; }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let conn = Connection::open("./todo.db")?;
    conn.execute("CREATE TABLE IF NOT EXISTS items (\
         ID INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,\
         ITEM TEXT NOT NULL UNIQUE,\
         DUE_DATE TEXT,\
         IS_COMPLETED INTEGER NOT NULL\
    )", ())?;
    match cli.command {
        Some(Commands::Add { item, }) => {
            add_item(item, &conn);
        }
        Some(Commands::Complete { id }) => {
            conn.execute("UPDATE items SET IS_COMPLETED = 1 WHERE ID = ?", params!(id))?;
        },
        Some(Commands::Remove { id }) => {
            conn.execute("DELETE FROM items WHERE ID = ?", params!(id))?;
        }
        None => {
            list_items(&conn);
        }
    }
    Ok(())
}