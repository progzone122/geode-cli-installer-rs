use colored::*;
use std::io::{self, Write};
use std::path::Path;
use std::process;

mod utils;
use utils::geode_installer::GeodeInstaller;

enum MenuChoice {
    InstallToSteam,
    InstallToWine,
    Quit,
}

struct UserInterface;

impl UserInterface {
    fn clear_screen() {
        let _ = process::Command::new("clear").status();
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
        println!("{} Install to {}", "1.".blue().bold(), "Steam".blue());
        println!("{} Install to {} prefix", "2.".magenta().bold(), "Wine".magenta());
        println!("{} Quit", "0.".red().bold());
        println!();
    }

    fn read_input(prompt: &str) -> String {
        print!("{}", prompt.white().bold());
        io::stdout().flush().expect("Failed to flush stdout");
        
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        
        input.trim().to_string()
    }

    fn read_menu_choice() -> Result<MenuChoice, ()> {
        let input = Self::read_input("What do you want to do: ");
        
        match input.parse::<i32>() {
            Ok(1) => Ok(MenuChoice::InstallToSteam),
            Ok(2) => Ok(MenuChoice::InstallToWine),
            Ok(0) => Ok(MenuChoice::Quit),
            _ => Err(()),
        }
    }

    fn print_success() {
        println!();
        println!("{}", "✅ Geode has been successfully installed!".green().bold());
    }

    fn print_error(message: &str) {
        println!();
        println!("{} {}", "❌ An error occurred:".red().bold(), message.red());
        println!();
        Self::read_input("Press Enter to continue...");
    }

    #[allow(unused)]
    fn print_invalid_input() {
        Self::clear_screen();
        println!("{}", "❌ Invalid input. Please enter a number.".red().bold());
        println!();
        Self::read_input("Press Enter to continue...");
    }

    fn print_invalid_choice() {
        Self::clear_screen();
        println!("{}", "❌ Invalid choice. Please try again.".red().bold());
        println!();
        Self::read_input("Press Enter to continue...");
    }
}

struct InstallationHandler {
    installer: GeodeInstaller,
}

impl InstallationHandler {
    fn new() -> Result<Self, String> {
        Ok(Self {
            installer: GeodeInstaller::new()?,
        })
    }

    fn handle_steam_installation(&self) -> Result<(), String> {
        println!("{}", "🎮 Installing to Steam...".blue().bold());
        self.installer.install_to_steam()
    }

    fn handle_wine_installation(&self) -> Result<(), String> {
        println!("{}", "🍷 Wine Installation".magenta().bold());
        
        let game_path = UserInterface::read_input("Enter your Geometry Dash path: ");
        let wine_prefix = UserInterface::read_input("Enter your Wine prefix path: ");
        
        self.installer.install_to_wine(
            Path::new(&wine_prefix),
            Path::new(&game_path),
        )
    }

    fn execute(&self, choice: MenuChoice) -> Result<(), String> {
        match choice {
            MenuChoice::InstallToSteam => self.handle_steam_installation(),
            MenuChoice::InstallToWine => self.handle_wine_installation(),
            MenuChoice::Quit => {
                println!("{}", "👋 Exiting...".yellow().bold());
                process::exit(0);
            }
        }
    }
}

fn run_interactive_loop(handler: &InstallationHandler) {
    loop {
        UserInterface::clear_screen();
        UserInterface::print_header();
        UserInterface::print_menu();

        let choice = match UserInterface::read_menu_choice() {
            Ok(c) => c,
            Err(_) => {
                UserInterface::print_invalid_choice();
                continue;
            }
        };

        match handler.execute(choice) {
            Ok(_) => {
                UserInterface::print_success();
                process::exit(0);
            }
            Err(e) => {
                UserInterface::print_error(&e);
            }
        }
    }
}

fn main() {
    let handler = match InstallationHandler::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!(
                "{} {}",
                "❌ Failed to initialize installer:".red().bold(),
                e.red()
            );
            process::exit(1);
        }
    };

    run_interactive_loop(&handler);
}