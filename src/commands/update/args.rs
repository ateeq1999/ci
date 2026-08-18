use clap::Args as ClapArgs;

#[derive(ClapArgs, Clone, Debug, Default)]
pub struct Args {
    /// Skip the confirmation prompt before replacing the running binary
    #[arg(long, short = 'y')]
    pub yes: bool,
}
