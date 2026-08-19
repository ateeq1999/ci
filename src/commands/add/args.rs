use clap::{Args as ClapArgs, Subcommand};

#[derive(ClapArgs, Clone, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Wire up class-validator request validation (ValidationPipe)
    Validation,
    /// Wire up Redis-backed caching (@nestjs/cache-manager)
    Caching,
}
