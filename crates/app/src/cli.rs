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
    #[command(subcommand)]
    Index(IndexCommand),
    Search(SearchArgs),
    Get(GetArgs),
    Embed(EmbedArgs),
    Hybrid(HybridArgs),
    #[command(subcommand)]
    Graph(GraphCommand),
}

#[derive(Debug, Subcommand)]
pub(crate) enum CollectionCommand {
    Create(CreateCollectionArgs),
    List(ListCollectionsArgs),
    Destroy(DestroyCollectionArgs),
    Add(AddFilesArgs),
    Update(UpdateCollectionArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum IndexCommand {
    Status(IndexStatusArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum GraphCommand {
    Neighbors(GraphNeighborsArgs),
}

#[derive(Debug, Args)]
pub(crate) struct GraphNeighborsArgs {
    #[arg(value_name = "ID")]
    pub(crate) node: String,
    #[arg(long, value_name = "NAME")]
    pub(crate) collection: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct IndexStatusArgs {
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    #[arg(value_name = "QUERY")]
    pub(crate) query: String,
    #[arg(long, value_name = "NAME")]
    pub(crate) collection: Option<String>,
    #[arg(long, value_name = "N", default_value_t = 10)]
    #[arg(value_parser = clap::value_parser!(u16).range(1..=100))]
    pub(crate) limit: u16,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct GetArgs {
    #[arg(value_name = "COLLECTION")]
    pub(crate) collection: String,
    #[arg(value_name = "NAME_OR_ID")]
    pub(crate) name_or_id: String,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct EmbedArgs {
    #[arg(long, value_name = "NAME")]
    pub(crate) collection: Option<String>,
    #[arg(long, value_name = "NAME")]
    pub(crate) model: Option<String>,
    #[arg(long, value_name = "NAME")]
    pub(crate) reranker: Option<String>,
    #[arg(long)]
    pub(crate) download: bool,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct HybridArgs {
    #[arg(value_name = "QUERY")]
    pub(crate) query: String,
    #[arg(long, value_name = "NAME")]
    pub(crate) collection: Option<String>,
    #[arg(long, value_name = "N", default_value_t = 10)]
    #[arg(value_parser = clap::value_parser!(u16).range(1..=100))]
    pub(crate) limit: u16,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) no_rerank: bool,
    #[arg(long, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
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
