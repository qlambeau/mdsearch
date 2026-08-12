use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "kv", version, about = "Local markdown knowledge search")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(subcommand)]
    Collection(CollectionCommand),
}

#[derive(Debug, Subcommand)]
pub(crate) enum CollectionCommand {
    Create(CreateCollectionArgs),
}

#[derive(Debug, Args)]
pub(crate) struct CreateCollectionArgs {
    #[arg(value_name = "NAME")]
    pub(crate) name: String,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}
