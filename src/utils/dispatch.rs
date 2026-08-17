use crate::{Cli, args::Commands};

pub fn dispatch(cli: &Cli) {
    match cli.command {
        Some(Commands::Run { port }) => {
            println!("Running on port: {:?}", port);
        }
        Some(Commands::Dev) => {
            use crate::commands::dev;
            dev();
        }
        None => {
            println!("Hello, {}!", cli.name.as_deref().unwrap_or("world"));
        }
    }
}
