use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "mdsearch",
    version,
    about = "Search local Markdown collections",
    long_about = "Index Markdown collections locally and retrieve grounded passages, files, and graph context.",
    after_help = "Example: mdsearch search \"rust ownership\" --collection Notes"
)]
pub(crate) struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "database file (default: ~/.mdsearch/collections.db)"
    )]
    pub(crate) database: Option<PathBuf>,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(
        subcommand,
        about = "Create, list, update, and delete collections",
        after_help = "Example: mdsearch collection list"
    )]
    Collection(CollectionCommand),
    #[command(
        subcommand,
        about = "Inspect built indexes",
        after_help = "Example: mdsearch index status"
    )]
    Index(IndexCommand),
    #[command(
        about = "Search indexed passages lexically",
        after_help = "Example: mdsearch search \"rust ownership\" --collection Notes"
    )]
    Search(SearchArgs),
    #[command(
        about = "Retrieve the full content of a stored file",
        after_help = "Example: mdsearch get Notes rust.md"
    )]
    Get(GetArgs),
    #[command(
        about = "Build semantic indexes for collections",
        after_help = "Example: mdsearch embed --collection Notes --download"
    )]
    Embed(EmbedArgs),
    #[command(subcommand, about = "List or change local semantic models")]
    Model(ModelCommand),
    #[command(
        about = "Search using lexical and semantic indexes",
        after_help = "Example: mdsearch hybrid \"borrowing rules\" --collection Notes"
    )]
    Hybrid(HybridArgs),
    #[command(
        subcommand,
        about = "Inspect entity graph relationships",
        after_help = "Example: mdsearch graph neighbors rust.md --collection Notes"
    )]
    Graph(GraphCommand),
    #[command(
        about = "Run a read-only GraphQL query against the entity graph",
        after_help = "Example: mdsearch context '{ node(kind: \"file\", key: \"rust.md\") { key } }' --collection Notes"
    )]
    Context(ContextArgs),
}

#[derive(Debug, Args)]
#[command(
    about = "Run a read-only GraphQL query against a collection graph",
    after_help = "Example: mdsearch context '{ node(kind: \"file\", key: \"rust.md\") { key } }' --collection Notes"
)]
pub(crate) struct ContextArgs {
    #[arg(
        value_name = "QUERY",
        help = "GraphQL query to execute against the selected collection"
    )]
    pub(crate) query: String,
    #[arg(
        long,
        value_name = "NAME",
        short = 'c',
        required = true,
        help = "collection containing the graph to query"
    )]
    pub(crate) collection: String,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CollectionCommand {
    #[command(
        about = "Create a collection and register source paths",
        after_help = "Example: mdsearch collection create Notes ~/vault"
    )]
    Create(CreateCollectionArgs),
    #[command(
        about = "List collections and registered sources",
        after_help = "Example: mdsearch collection list --json"
    )]
    List(ListCollectionsArgs),
    #[command(
        about = "Replace a collection's registered sources",
        after_help = "Example: mdsearch collection configure Notes --sources ~/vault"
    )]
    Configure(ConfigureCollectionArgs),
    #[command(
        about = "Delete a collection and its stored indexes",
        after_help = "Example: mdsearch collection destroy Notes"
    )]
    Destroy(DestroyCollectionArgs),
    #[command(
        about = "Store Markdown files without indexing",
        after_help = "Example: mdsearch collection add Notes ~/vault"
    )]
    Add(AddFilesArgs),
    #[command(
        about = "Reconcile files and rebuild lexical/graph indexes",
        after_help = "Example: mdsearch collection update --collection Notes"
    )]
    Update(UpdateCollectionArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum IndexCommand {
    #[command(
        about = "Report lexical and semantic index readiness",
        after_help = "Example: mdsearch index status"
    )]
    Status(IndexStatusArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum ModelCommand {
    #[command(about = "List supported models and local availability")]
    List(ModelListArgs),
    #[command(about = "Change the database-wide embedding or re-ranker model")]
    Set(ModelSetArgs),
}

#[derive(Debug, Args)]
pub(crate) struct ModelListArgs {
    #[arg(long, help = "write model availability as JSON")]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ModelSetArgs {
    #[arg(value_name = "NAME", help = "embedding model name")]
    pub(crate) model: Option<String>,
    #[arg(long, value_name = "NAME", help = "re-ranker model name")]
    pub(crate) reranker: Option<String>,
    #[arg(long, help = "download model assets if unavailable locally")]
    pub(crate) download: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum GraphCommand {
    #[command(
        about = "Show relationships around a graph node",
        after_help = "Example: mdsearch graph neighbors rust.md --collection Notes"
    )]
    Neighbors(GraphNeighborsArgs),
}

#[derive(Debug, Args)]
pub(crate) struct GraphNeighborsArgs {
    #[arg(value_name = "ID", help = "node key to inspect")]
    pub(crate) node: String,
    #[arg(
        short = 'c',
        long,
        value_name = "NAME",
        help = "restrict graph lookup to this collection"
    )]
    pub(crate) collection: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct IndexStatusArgs {}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    #[arg(value_name = "QUERY", help = "literal search terms")]
    pub(crate) query: String,
    #[arg(
        short = 'c',
        long,
        value_name = "NAME",
        help = "restrict search to this collection (default: all collections)"
    )]
    pub(crate) collection: Option<String>,
    #[arg(
        short = 'n',
        long,
        value_name = "N",
        default_value_t = 10,
        help = "maximum results (1–100, default: 10)"
    )]
    #[arg(value_parser = clap::value_parser!(u16).range(1..=100))]
    pub(crate) limit: u16,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) related: bool,
}

#[derive(Debug, Args)]
pub(crate) struct GetArgs {
    #[arg(value_name = "COLLECTION", help = "collection containing the file")]
    pub(crate) collection: String,
    #[arg(value_name = "NAME_OR_ID", help = "stored file name, path, or ID")]
    pub(crate) name_or_id: String,
}

#[derive(Debug, Args)]
pub(crate) struct EmbedArgs {
    #[arg(
        short = 'c',
        long,
        value_name = "NAME",
        help = "restrict embedding to this collection (default: all collections)"
    )]
    pub(crate) collection: Option<String>,
    #[arg(long, value_name = "NAME", help = "embedding model name")]
    pub(crate) model: Option<String>,
    #[arg(long, value_name = "NAME", help = "re-ranker model name")]
    pub(crate) reranker: Option<String>,
    #[arg(
        long,
        help = "download model assets if not already downloaded (stored under ~/.mdsearch/models by default)"
    )]
    pub(crate) download: bool,
}

#[derive(Debug, Args)]
pub(crate) struct HybridArgs {
    #[arg(value_name = "QUERY", help = "literal search terms")]
    pub(crate) query: String,
    #[arg(
        short = 'c',
        long,
        value_name = "NAME",
        help = "restrict search to this collection (default: all collections)"
    )]
    pub(crate) collection: Option<String>,
    #[arg(
        short = 'n',
        long,
        value_name = "N",
        default_value_t = 10,
        help = "maximum results (1–100, default: 10)"
    )]
    #[arg(value_parser = clap::value_parser!(u16).range(1..=100))]
    pub(crate) limit: u16,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) related: bool,
    #[arg(long)]
    pub(crate) no_rerank: bool,
}

#[derive(Debug, Args)]
pub(crate) struct CreateCollectionArgs {
    #[arg(value_name = "NAME", help = "new collection name")]
    pub(crate) name: String,
    #[arg(value_name = "PATH", num_args = 1.., help = "Markdown file or directory source")]
    pub(crate) paths: Vec<PathBuf>,
    #[arg(long, help = "enable semantic indexing for this collection")]
    pub(crate) semantic: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ListCollectionsArgs {
    #[arg(long, help = "write collection names and sources as JSON")]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ConfigureCollectionArgs {
    #[arg(value_name = "NAME", help = "collection to configure")]
    pub(crate) name: String,
    #[arg(long = "sources", value_name = "PATH", num_args = 1.., help = "replacement Markdown file or directory sources")]
    pub(crate) paths: Vec<PathBuf>,
    #[arg(long, value_name = "on|off", value_parser = ["on", "off"], help = "enable or disable semantic indexing")]
    pub(crate) semantic: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct DestroyCollectionArgs {
    #[arg(value_name = "NAME", help = "collection to delete")]
    pub(crate) name: String,
}

#[derive(Debug, Args)]
pub(crate) struct AddFilesArgs {
    #[arg(value_name = "NAME", help = "existing collection to receive the files")]
    pub(crate) name: String,
    #[arg(value_name = "PATH", required = true, num_args = 1..)]
    pub(crate) paths: Vec<PathBuf>,
    #[arg(long, help = "skip unreadable files and continue ingestion")]
    pub(crate) skip_unreadable: bool,
}

#[derive(Debug, Args)]
pub(crate) struct UpdateCollectionArgs {
    #[arg(long)]
    pub(crate) all: bool,
    #[arg(
        long = "collection",
        short = 'c',
        value_name = "NAME",
        conflicts_with = "all"
    )]
    pub(crate) name: Option<String>,
    #[arg(value_name = "NAME", conflicts_with = "all")]
    pub(crate) legacy_name: Option<String>,
    #[arg(value_name = "PATH", num_args = 1..)]
    pub(crate) legacy_paths: Vec<PathBuf>,
    #[arg(long, help = "skip unreadable files and continue updating")]
    pub(crate) skip_unreadable: bool,
    #[arg(
        long,
        help = "download model assets required for enabled semantic indexes"
    )]
    pub(crate) download: bool,
}
