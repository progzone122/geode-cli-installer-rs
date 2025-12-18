use colored::*;
use std::io::{self, Write};
use std::process::Command;

mod utils;

use utils::geode_installer;

use geode_installer::GeodeInstaller;

fn clear_screen() {
    Command::new("clear").status().ok();
}

fn print_header() {
    println!("{}", "======================================".yellow().bold());
    println!("{}", "       Geode Installer for Linux     ".yellow().bold());
    println!("{}", "======================================".yellow().bold());
    println!();
}

fn print_menu() {
    println!("{}", "Select an action:".white().bold());
    println!();
    println!(
        "{} Install to {}",
        "1.".blue().bold(),
        "Steam".blue()
    );
    println!(
        "{} Install to {} prefix",
        "2.".magenta().bold(),
        "Wine".magenta()
    );
    println!("{} Quit", "0.".red().bold());
    println!();
}

fn read_input(prompt: &str) -> String {
    print!("{}", prompt.white().bold());
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    
    input.trim().to_string()
}

fn install_to_steam(installer: &GeodeInstaller) -> Result<(), String> {
    println!("{}", "🎮 Installing to Steam...".blue().bold());
    installer.install_geode_to_steam()
}

fn install_to_wine(installer: &GeodeInstaller) -> Result<(), String> {
    println!("{}", "🍷 Wine Installation".magenta().bold());
    
    let gd_path = read_input(&format!(
        "Enter your Geometry Dash path: {}",
        "".yellow()
    ));
    
    let wine_prefix = read_input(&format!(
        "Enter your {} prefix path: ",
        "Wine".magenta()
    ));
    
    installer.install_geode_to_wine(
        std::path::Path::new(&wine_prefix),
        std::path::Path::new(&gd_path),
    )
}

fn main() {
    clear_screen();

    let installer = match GeodeInstaller::new() {
        Ok(inst) => inst,
        Err(e) => {
            eprintln!("{} {}", "❌ Failed to initialize installer. This is so bad:".red().bold(), e.red());
            std::process::exit(1);
        }
    };

    loop {
        print_header();
        print_menu();

        let input = read_input("What do you want to do: ");

        let choice: i32 = match input.parse() {
            Ok(num) => num,
            Err(_) => {
                clear_screen();
                println!(
                    "{}",
                    "❌ Invalid input. Please enter a number.".red().bold()
                );
                println!();
                continue;
            }
        };

        let result = match choice {
            1 => install_to_steam(&installer),
            2 => install_to_wine(&installer),
            0 => {
                println!("{}", "👋 Exiting...".yellow().bold());
                std::process::exit(0);
            }
            _ => {
                clear_screen();
                println!(
                    "{}",
                    "❌ Invalid choice. Please try again.".red().bold()
                );
                println!();
                continue;
            }
        };

        match result {
            Ok(_) => {
                println!();
                println!(
                    "{}",
                    "✅ Geode has been successfully installed!".green().bold()
                );
                std::process::exit(0);
            }
            Err(e) => {
                println!();
                println!(
                    "{} {}",
                    "❌ An error occurred:".red().bold(),
                    e.red()
                );
                println!();
                continue;
            }
        }
    }
}
