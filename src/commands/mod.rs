pub mod db;
pub mod init;
pub mod update;

use crate::args::{Cli, Commands};
use crate::context::Context;
use crate::json_payload;

pub fn run(cli: &Cli) -> anyhow::Result<()> {
    let ctx = Context::real();

    match &cli.command {
        Commands::Init(args) => {
            let args = json_payload::resolve(args.clone(), cli.json.as_deref())?;
            init::run(&args, &ctx)
        }
        Commands::Update(args) => update::run(args),
        Commands::Db(args) => db::run(args, &ctx),
    }
}
