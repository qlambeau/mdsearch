use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "mdsearch", version, about = "Local markdown knowledge search")]
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
    List(ListCollectionsArgs),
    Destroy(DestroyCollectionArgs),
    Add(AddFilesArgs),
    Update(UpdateCollectionArgs),
}

#[derive(Debug, Args)]
pub(crate) struct CreateCollectionArgs {
    #[arg(value_name = "NAME")]
    pub(crate) name: String,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ListCollectionsArgs {
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct DestroyCollectionArgs {
    #[arg(value_name = "NAME")]
    pub(crate) name: String,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct AddFilesArgs {
    #[arg(value_name = "NAME")]
    pub(crate) name: String,
    #[arg(value_name = "PATH", required = true, num_args = 1..)]
    pub(crate) paths: Vec<PathBuf>,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
    #[arg(long)]
    pub(crate) force: bool,
}

#[derive(Debug, Args)]
pub(crate) struct UpdateCollectionArgs {
    #[arg(long)]
    pub(crate) all: bool,
    #[arg(
        value_name = "NAME",
        required_unless_present = "all",
        conflicts_with = "all"
    )]
    pub(crate) name: Option<String>,
    #[arg(
        value_name = "PATH",
        num_args = 1..,
        required_unless_present = "all",
        conflicts_with = "all"
    )]
    pub(crate) paths: Vec<PathBuf>,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
    #[arg(long)]
    pub(crate) force: bool,
}
