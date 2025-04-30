mod fetch;

use dialoguer::{Select, theme::ColorfulTheme};

fn main() {
    let menu_options = ["FETCH", "EXIT"];

    loop {
        let selected_index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("bulba-utils v1.0.0")
            .default(0)
            .items(&menu_options)
            .report(false)
            .interact()
            .unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                menu_options.len() - 1
            });

        match menu_options[selected_index] {
            "FETCH" => fetch::fetch(),
            "EXIT" => break,
            _ => continue,
        }
    }
}
