mod init;

use crate::args::{Cli, Commands};
use crate::fs::RealFileSystem;
use crate::json_payload;

pub fn run(cli: &Cli) -> anyhow::Result<()> {
    match &cli.command {
        Commands::Init(args) => {
            let args = json_payload::resolve(args.clone(), cli.json.as_deref())?;
            init::run(&args, &RealFileSystem)
        }
    }
}
