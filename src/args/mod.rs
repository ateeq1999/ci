use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Run the application
    Run {
        /// The port to run on
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Run the application in development mode
    Dev,
}
