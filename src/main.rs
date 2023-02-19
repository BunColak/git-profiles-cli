use clap::{Parser, Subcommand};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, Table};
use rusqlite::{Connection, Result};
use std::process::Command;
use colored::Colorize;

#[derive(Debug)]
struct GitProfile {
    name: String,
    email: String,
    alias: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List registered Git profiles
    List {},
    /// Add a new Git profile
    Add {
        /// Your name for this profile (user.name)
        #[arg(short, long)]
        name: String,

        /// Your email for this profile (user.email)
        #[arg(short, long)]
        email: String,

        /// You will use this name for switching in between
        #[arg(short, long)]
        alias: String,
    },
    /// Switch between Git profiles
    Switch {
        #[arg(short, long)]
        alias: Option<String>,

        #[arg(short, long)]
        email: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let conn = Connection::open("profiles.db")?;

    conn.execute(
        "create table IF NOT EXISTS profiles (
                        id integer primary key,
                        email TEXT not null unique,
                        name TEXT not null,
                        alias TEXT unique
                         )",
        [],
    )
        .unwrap();

    get_current_profile();

    match cli.command {
        Commands::List {} => {
            list_profiles(&conn).unwrap();
        }
        Commands::Add { name, alias, email } => {
            add_profile(&conn, &name, &alias, &email).unwrap();
        }
        Commands::Switch { alias, email } => {
            // Get from database
            // Set current profile
            set_profile_from_db(&conn, alias, email)
        }
    }

    Ok(())
}

fn set_profile_from_db(conn: &Connection, alias: Option<String>, email: Option<String>) {
    let mut profiles_query = conn
        .prepare(
            "select alias, email, name from profiles where alias like :alias or email = :email limit 1;",
        )
        .unwrap();

    let alias_str = alias.unwrap_or_default();

    let profiles = profiles_query
        .query_map(
            rusqlite::named_params! {
            ":alias": "%".to_owned() + &alias_str + "%",
            ":email": email.unwrap_or_default()},
            |row| {
                Ok(GitProfile {
                    alias: row.get(0)?,
                    email: row.get(1)?,
                    name: row.get(2)?,
                })
            },
        )
        .unwrap();

    for profile in profiles {
        let profile = profile.unwrap();
        change_profile(profile).expect("Error changing profiles");
    }
}

fn add_profile(conn: &Connection, name: &str, alias: &str, email: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO profiles (email, name, alias) VALUES (?1, ?2, ?3)",
        (email, name, alias),
    )
        .expect("Error adding new profile to DB.");

    println!("Successfully added new profile!");
    Ok(())
}

fn change_profile(profile: GitProfile) -> Result<()> {

    Command::new("git")
        .arg("config")
        .arg("--global")
        .arg("user.email")
        .arg(&profile.email)
        .output()
        .expect("Error setting the email");

    Command::new("git")
        .arg("config")
        .arg("--global")
        .arg("user.name")
        .arg(&profile.name)
        .output()
        .expect("Error setting the name");

    println!("Successfully changed profile to {} ({})", profile.alias, profile.email);

    Ok(())
}

fn get_current_profile() -> String {
    let output = Command::new("git")
        .arg("config")
        .arg("--global")
        .arg("--get")
        .arg("user.email")
        .output()
        .expect("Error getting the email");

    return String::from_utf8(output.stdout).unwrap().trim().to_string();
}

fn list_profiles(conn: &Connection) -> Result<()> {
    let mut profiles_query = conn
        .prepare("select alias, email, name from profiles;")
        .unwrap(); 

    let profiles = profiles_query
        .query_map([], |row| {
            Ok(GitProfile {
                alias: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
            })
        })
        .unwrap();

    let current_email = get_current_profile();

    // Table to display data
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Alias").add_attribute(Attribute::Bold),
        Cell::new("Name").add_attribute(Attribute::Bold),
        Cell::new("Email").add_attribute(Attribute::Bold),
    ]);

    for profile in profiles {
        let resolved_profile = profile.unwrap();

        let is_current_profile = resolved_profile.email.as_str() == current_email.as_str();
        let cells = vec![
            Cell::new(resolved_profile.alias).fg(Color::DarkYellow),
            Cell::new(resolved_profile.name).fg(Color::Green),
            Cell::new(resolved_profile.email).fg(Color::Blue),
        ];

        if is_current_profile {
            let mut new_cells: Vec<Cell> = vec![];
            for x in &cells {
                new_cells.push(x.clone().add_attribute(Attribute::Bold).add_attribute(Attribute::Italic));
            }
            table.add_row(new_cells);
        } else {
            table.add_row(cells);
        }
    }

    println!("{table}");

    Ok(())
}
