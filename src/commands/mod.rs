mod dev;

use crate::args::{Cli, Commands};

pub fn run(cli: &Cli) -> anyhow::Result<()> {
    match &cli.command {
        Some(Commands::Run { port }) => {
            println!("Running on port: {:?}", port);
        }
        Some(Commands::Dev) => {
            dev::dev();
        }
        None => {
            println!("Hello, {}!", cli.name.as_deref().unwrap_or("world"));
        }
    }

    Ok(())
}
