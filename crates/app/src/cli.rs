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
        about = "Register, configure, list, and delete collections",
        after_help = "Example: mdsearch collection list"
    )]
    Collection(CollectionCommand),
    #[command(
        about = "Refresh every configured index from registered sources",
        after_help = "Example: mdsearch update -c Notes --json"
    )]
    Update(UpdateCollectionArgs),
    #[command(
        about = "Inspect enabled indexes, stored-content freshness, and models",
        after_help = "Example: mdsearch status -c Notes --json"
    )]
    Status(IndexStatusArgs),
    #[command(
        about = "Search indexed passages (default: lexical; hybrid uses local models)",
        after_help = "Example: mdsearch search \"rust ownership\" --collection Notes"
    )]
    Search(SearchArgs),
    #[command(
        about = "Retrieve the full content of a stored file",
        after_help = "Example: mdsearch get rust.md -c Notes"
    )]
    Get(GetArgs),
    #[command(
        subcommand,
        about = "List or change local semantic models",
        after_help = "Example: mdsearch model list --json"
    )]
    Model(ModelCommand),
    #[command(
        subcommand,
        about = "Inspect entity graph relationships",
        after_help = "Example: mdsearch graph neighbors rust.md --collection Notes"
    )]
    Graph(GraphCommand),
}

#[derive(Debug, Args)]
#[command(
    about = "Run a read-only GraphQL query against a collection graph",
    after_help = "Example: mdsearch graph query '{ node(kind: \"file\", key: \"rust.md\") { key } }' --collection Notes"
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
        about = "List registered sources and enabled indexes",
        after_help = "Example: mdsearch collection list --json"
    )]
    List(ListCollectionsArgs),
    #[command(
        about = "Replace registered sources and/or change semantic policy without indexing",
        after_help = "Example: mdsearch collection configure Notes --sources ~/vault"
    )]
    Configure(ConfigureCollectionArgs),
    #[command(
        about = "Delete a collection and its stored indexes",
        after_help = "Example: mdsearch collection delete Notes"
    )]
    Delete(DestroyCollectionArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum ModelCommand {
    #[command(
        about = "List supported models and local availability",
        after_help = "Example: mdsearch model list --json"
    )]
    List(ModelListArgs),
    #[command(
        about = "Change database-wide models and atomically rebuild enabled semantic indexes",
        after_help = "Example: mdsearch model set all-MiniLM-L6-v2 --download"
    )]
    Set(ModelSetArgs),
}

#[derive(Debug, Args)]
pub(crate) struct ModelListArgs {
    #[arg(long, help = "write model availability as JSON")]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("models").required(true).multiple(true).args(["model", "reranker"])))]
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
    #[command(
        about = "Execute GraphQL bound to the selected collection",
        after_help = "Example: mdsearch graph query '{ node(kind: \"file\", key: \"rust.md\") { key } }' -c Notes"
    )]
    Query(ContextArgs),
}

#[derive(Debug, Args)]
pub(crate) struct GraphNeighborsArgs {
    #[arg(value_name = "KEY", help = "exact graph node key to inspect")]
    pub(crate) node: String,
    #[arg(
        short = 'c',
        long,
        value_name = "NAME",
        required = true,
        help = "collection containing the graph"
    )]
    pub(crate) collection: String,
    #[arg(long, default_value = "file", value_parser = ["file", "tag", "alias"], help = "node kind (default: file)")]
    pub(crate) kind: String,
    #[arg(long, value_parser = ["LINKS_TO", "TAGGED_WITH", "ALIAS_OF", "RELATED_TO", "HAS_SOURCE"], help = "restrict traversed relation kind")]
    pub(crate) relation: Option<String>,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=255), help = "maximum traversal depth (1–255, default: 1)")]
    pub(crate) depth: u8,
}

#[derive(Debug, Args)]
pub(crate) struct IndexStatusArgs {
    #[arg(
        short = 'c',
        long,
        value_name = "NAME",
        help = "restrict status to one collection"
    )]
    pub(crate) collection: Option<String>,
    #[arg(long, help = "write index status as JSON")]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    #[arg(value_name = "QUERY", required = true, num_args = 1.., help = "literal query words (quoted or separate)")]
    pub(crate) query: Vec<String>,
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
    #[arg(long, help = "write structured search results as JSON")]
    pub(crate) json: bool,
    #[arg(long, help = "include related files without changing ranking")]
    pub(crate) related: bool,
    #[arg(long, value_enum, default_value_t = SearchMode::Lexical, help = "retrieval mode (default: lexical)")]
    pub(crate) mode: SearchMode,
    #[arg(long, help = "disable re-ranking in hybrid mode")]
    pub(crate) no_rerank: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum SearchMode {
    Lexical,
    Hybrid,
}

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("selector").required(true).args(["path_or_name", "id"])))]
pub(crate) struct GetArgs {
    #[arg(
        short = 'c',
        long,
        required = true,
        value_name = "NAME",
        help = "collection containing the file"
    )]
    pub(crate) collection: String,
    #[arg(
        value_name = "PATH_OR_NAME",
        help = "exact stored path or unique basename (numeric names remain names)"
    )]
    pub(crate) path_or_name: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..=i64::MAX as u64), help = "explicit stored file ID")]
    pub(crate) id: Option<u64>,
}

#[derive(Debug, Args)]
pub(crate) struct CreateCollectionArgs {
    #[arg(value_name = "NAME", help = "new collection name")]
    pub(crate) name: String,
    #[arg(value_name = "PATH", required = true, num_args = 1.., help = "Markdown file or directory source")]
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
#[command(group(clap::ArgGroup::new("configuration").required(true).multiple(true).args(["paths", "semantic"])))]
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
#[command(group(clap::ArgGroup::new("scope").required(true).args(["all", "name"])))]
pub(crate) struct UpdateCollectionArgs {
    #[command(flatten)]
    pub(crate) scope: UpdateScopeArgs,
    #[arg(long, help = "skip unreadable files, retaining their stored content")]
    pub(crate) skip_unreadable: bool,
    #[arg(
        long,
        help = "download missing model assets required by enabled semantic indexes"
    )]
    pub(crate) download: bool,
    #[arg(long, help = "write a complete per-collection JSON report")]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct UpdateScopeArgs {
    #[arg(long, help = "update every collection, continuing after failures")]
    pub(crate) all: bool,
    #[arg(
        short = 'c',
        long = "collection",
        value_name = "NAME",
        help = "collection to update"
    )]
    pub(crate) name: Option<String>,
}
