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
    Cache,
    /// Wire up task scheduling (@nestjs/schedule)
    Schedule,
    /// Wire up Redis-backed queues (@nestjs/bullmq)
    Queue,
    /// Wire up a standalone LoggerModule (console logger by default)
    Logger,
    /// Wire up the event emitter (@nestjs/event-emitter)
    Events,
}
