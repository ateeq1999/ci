pub mod db;
pub mod init;
pub mod update;

use crate::args::{Cli, Commands};
use crate::shared::context::Context;
use crate::shared::json_payload;

pub fn run(cli: &Cli, ctx: &Context) -> anyhow::Result<()> {
    match &cli.command {
        Commands::Init(args) => {
            let args = json_payload::resolve(args.clone(), cli.json.as_deref())?;
            init::run(&args, ctx)
        }
        Commands::Update(args) => update::run(args, ctx),
        Commands::Db(args) => db::run(args, ctx),
    }
}
