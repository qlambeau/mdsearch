//! Deterministic contextual entity-graph extraction.
//!
//! `extract_graph` derives an [`EntityGraph`] from a collection's stored files
//! without any LLM or network access. File nodes use the indexing-assigned
//! stable file ID; tag and alias nodes use their exact normalized name. Edges
//! are typed and directional and drawn from frontmatter fields (`tags:`,
//! `aliases:`, `related:`, `sources:`), inline relative `.md` links, and
//! Obsidian-style wikilinks (`[[target]]`, `[[target|label]]`,
//! `[[path/target#heading]]`).

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use yaml_rust2::Yaml;
use yaml_rust2::YamlLoader;

use crate::FileId;

/// The kind of an entity node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EntityKind {
    /// A stored markdown file.
    File,
    /// A frontmatter `tags:` value.
    Tag,
    /// A frontmatter `aliases:` value.
    Alias,
}

impl EntityKind {
    /// Returns the stable database key for this node kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Tag => "tag",
            Self::Alias => "alias",
        }
    }

    /// Reconstructs a node kind from its stable database key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "file" => Some(Self::File),
            "tag" => Some(Self::Tag),
            "alias" => Some(Self::Alias),
            _ => None,
        }
    }
}

/// The typed, directional relationship between two entity nodes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelationKind {
    /// An inline relative `.md` link from one file to another.
    LinksTo,
    /// A file is tagged with a tag node.
    TaggedWith,
    /// A file has an alias node.
    AliasOf,
    /// A frontmatter `related:` reference from one file to another.
    RelatedTo,
    /// A frontmatter `sources:` reference from one file to another.
    HasSource,
}

impl RelationKind {
    /// Returns the stable database key for this relation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinksTo => "LINKS_TO",
            Self::TaggedWith => "TAGGED_WITH",
            Self::AliasOf => "ALIAS_OF",
            Self::RelatedTo => "RELATED_TO",
            Self::HasSource => "HAS_SOURCE",
        }
    }

    /// Reconstructs a relation from its stable database key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "LINKS_TO" => Some(Self::LinksTo),
            "TAGGED_WITH" => Some(Self::TaggedWith),
            "ALIAS_OF" => Some(Self::AliasOf),
            "RELATED_TO" => Some(Self::RelatedTo),
            "HAS_SOURCE" => Some(Self::HasSource),
            _ => None,
        }
    }
}

/// The stable identity of a graph node within a collection.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId {
    kind: EntityKind,
    key: String,
}

impl NodeId {
    /// Creates a node identity from its kind and stable key.
    #[must_use]
    pub fn new(kind: EntityKind, key: String) -> Self {
        Self { kind, key }
    }

    /// Returns the node kind.
    #[must_use]
    pub const fn kind(&self) -> EntityKind {
        self.kind
    }

    /// Returns the stable node key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// A graph node with its human label.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GraphNode {
    id: NodeId,
    title: String,
}

impl GraphNode {
    /// Creates a node from its identity and title.
    #[must_use]
    pub fn new(id: NodeId, title: String) -> Self {
        Self { id, title }
    }

    /// Returns the node identity.
    #[must_use]
    pub const fn id(&self) -> &NodeId {
        &self.id
    }

    /// Returns the human label.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// A typed, directional edge between two node identities.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GraphEdge {
    src: NodeId,
    dst: NodeId,
    relation: RelationKind,
}

impl GraphEdge {
    /// Creates an edge from its endpoints and relation.
    #[must_use]
    pub fn new(src: NodeId, dst: NodeId, relation: RelationKind) -> Self {
        Self { src, dst, relation }
    }

    /// Returns the source node identity.
    #[must_use]
    pub const fn src(&self) -> &NodeId {
        &self.src
    }

    /// Returns the destination node identity.
    #[must_use]
    pub const fn dst(&self) -> &NodeId {
        &self.dst
    }

    /// Returns the relation kind.
    #[must_use]
    pub const fn relation(&self) -> RelationKind {
        self.relation
    }
}

/// A collection's deterministic entity graph: a set of nodes and typed edges.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntityGraph {
    nodes: BTreeSet<GraphNode>,
    edges: BTreeSet<GraphEdge>,
}

impl EntityGraph {
    /// Creates an empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a node, returning whether it was newly added.
    pub fn insert_node(&mut self, node: GraphNode) -> bool {
        self.nodes.insert(node)
    }

    /// Inserts an edge, returning whether it was newly added.
    pub fn insert_edge(&mut self, edge: GraphEdge) -> bool {
        self.edges.insert(edge)
    }

    /// Returns the nodes in deterministic order.
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.iter()
    }

    /// Returns the node with the given identity, if present.
    #[must_use]
    pub fn node(&self, id: &NodeId) -> Option<&GraphNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    /// Returns the edges in deterministic order.
    pub fn edges(&self) -> impl Iterator<Item = &GraphEdge> {
        self.edges.iter()
    }

    /// Returns the number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// A file supplied for graph extraction: its stable ID, path, and content.
pub struct GraphSource<'a> {
    file_id: FileId,
    path: &'a Path,
    content: &'a [u8],
}

impl<'a> GraphSource<'a> {
    /// Creates a graph source from its identity, path, and content.
    #[must_use]
    pub fn new(file_id: FileId, path: &'a Path, content: &'a [u8]) -> Self {
        Self {
            file_id,
            path,
            content,
        }
    }

    /// Returns the stable file ID.
    #[must_use]
    pub const fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Returns the file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.path
    }

    /// Returns the file content.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        self.content
    }
}

/// Extracts the deterministic entity graph for a collection from its stored
/// files.
///
/// The function is pure and deterministic: the same file set always yields the
/// same [`EntityGraph`]. Unknown `related:`/`sources:` references and inline
/// link targets that do not match a known file are skipped.
#[must_use]
pub fn extract_graph(files: &[GraphSource<'_>]) -> EntityGraph {
    let known_paths: HashSet<String> = files
        .iter()
        .map(|file| file.path().to_string_lossy().into_owned())
        .collect();

    let mut graph = EntityGraph::new();

    for file in files {
        let path = file.path().to_string_lossy().into_owned();
        let title = file
            .path()
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());

        let file_node = GraphNode::new(NodeId::new(EntityKind::File, path), title);
        graph.insert_node(file_node);

        let frontmatter = parse_frontmatter(file.content());

        // Tags -> TAGGED_WITH edges.
        for tag in frontmatter_values(frontmatter.as_ref(), "tags") {
            let key = normalize_name(&tag);
            graph.insert_node(GraphNode::new(
                NodeId::new(EntityKind::Tag, key.clone()),
                key.clone(),
            ));
            graph.insert_edge(GraphEdge::new(
                NodeId::new(EntityKind::File, file.path().to_string_lossy().into_owned()),
                NodeId::new(EntityKind::Tag, key),
                RelationKind::TaggedWith,
            ));
        }

        // Aliases -> ALIAS_OF edges.
        for alias in frontmatter_values(frontmatter.as_ref(), "aliases") {
            let key = normalize_name(&alias);
            graph.insert_node(GraphNode::new(
                NodeId::new(EntityKind::Alias, key.clone()),
                key.clone(),
            ));
            graph.insert_edge(GraphEdge::new(
                NodeId::new(EntityKind::File, file.path().to_string_lossy().into_owned()),
                NodeId::new(EntityKind::Alias, key),
                RelationKind::AliasOf,
            ));
        }

        // related: -> RELATED_TO edges (skip unresolved).
        for target in frontmatter_values(frontmatter.as_ref(), "related") {
            if let Some(resolved) = resolve_file(&target, file.path(), &known_paths) {
                graph.insert_edge(GraphEdge::new(
                    NodeId::new(EntityKind::File, file.path().to_string_lossy().into_owned()),
                    NodeId::new(EntityKind::File, resolved),
                    RelationKind::RelatedTo,
                ));
            }
        }

        // sources: -> HAS_SOURCE edges (skip unresolved).
        for target in frontmatter_values(frontmatter.as_ref(), "sources") {
            if let Some(resolved) = resolve_file(&target, file.path(), &known_paths) {
                graph.insert_edge(GraphEdge::new(
                    NodeId::new(EntityKind::File, file.path().to_string_lossy().into_owned()),
                    NodeId::new(EntityKind::File, resolved),
                    RelationKind::HasSource,
                ));
            }
        }

        // Inline relative .md links -> LINKS_TO edges (skip unresolved).
        for target in inline_markdown_links(file.content()) {
            if let Some(resolved) = resolve_file(&target, file.path(), &known_paths) {
                graph.insert_edge(GraphEdge::new(
                    NodeId::new(EntityKind::File, file.path().to_string_lossy().into_owned()),
                    NodeId::new(EntityKind::File, resolved),
                    RelationKind::LinksTo,
                ));
            }
        }

        // Wikilinks -> LINKS_TO edges (skip unresolved, ambiguous, self-edges).
        insert_wikilink_edges(&mut graph, file, &known_paths);
    }

    graph
}

/// Inserts `LINKS_TO` edges for a file's wikilinks, skipping unresolved,
/// ambiguous, and self-referential targets.
fn insert_wikilink_edges(
    graph: &mut EntityGraph,
    file: &GraphSource<'_>,
    known_paths: &HashSet<String>,
) {
    for target in inline_wikilinks(file.content()) {
        if let Some(resolved) = resolve_wikilink(&target, file.path(), known_paths) {
            if resolved == file.path().to_string_lossy() {
                continue;
            }
            graph.insert_edge(GraphEdge::new(
                NodeId::new(EntityKind::File, file.path().to_string_lossy().into_owned()),
                NodeId::new(EntityKind::File, resolved),
                RelationKind::LinksTo,
            ));
        }
    }
}

/// Normalizes a tag or alias name to its canonical key.
fn normalize_name(value: &str) -> String {
    value.trim().to_owned()
}

/// Parses the YAML frontmatter block of `content`, returning the mapping or
/// `None` when there is no frontmatter or it is malformed.
fn parse_frontmatter(content: &[u8]) -> Option<Yaml> {
    let text = String::from_utf8_lossy(content);
    let mut lines = text.lines();
    if lines.next().is_none_or(|line| line.trim() != "---") {
        return None;
    }
    let mut block = String::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        block.push_str(line);
        block.push('\n');
    }
    let docs = YamlLoader::load_from_str(&block).ok()?;
    docs.into_iter().next()
}

/// Returns the scalar or list values of a frontmatter field as trimmed strings.
fn frontmatter_values(frontmatter: Option<&Yaml>, key: &str) -> Vec<String> {
    let Some(yaml) = frontmatter else {
        return Vec::new();
    };
    let value = &yaml[key];
    if value.is_badvalue() {
        return Vec::new();
    }
    let mut values = Vec::new();
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            values.push(trimmed.to_owned());
        }
    } else if let Some(items) = value.as_vec() {
        for item in items {
            if let Some(text) = item.as_str() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    values.push(trimmed.to_owned());
                }
            }
        }
    }
    values
}

/// Resolves a reference (inline link target or `related:`/`sources:` entry) to a
/// known file path within the collection.
///
/// Resolution tries, in order: the reference as-is, the reference joined onto
/// the source file's directory, and finally a basename match (only when exactly
/// one known file shares the basename). Absolute URLs and anchors never resolve.
fn resolve_file(reference: &str, source_path: &Path, known: &HashSet<String>) -> Option<String> {
    let target = reference.trim();
    if target.is_empty() || target.starts_with("http://") || target.starts_with("https://") {
        return None;
    }

    let mut candidates = reference_candidates(target, source_path);
    if !has_markdown_extension(target) {
        let appended = format!("{target}.md");
        candidates.push(appended.clone());
        if let Some(parent) = source_path.parent() {
            candidates.push(parent.join(&appended).to_string_lossy().into_owned());
        }
    }

    for candidate in &candidates {
        if known.contains(candidate) {
            return Some(candidate.clone());
        }
    }

    let mut basenames = vec![Path::new(target).to_path_buf()];
    if !has_markdown_extension(target) {
        basenames.push(Path::new(&format!("{target}.md")).to_path_buf());
    }
    for basename in basenames {
        if let Some(basename) = basename.file_name() {
            let basename = basename.to_string_lossy();
            let matches: Vec<&String> = known
                .iter()
                .filter(|path| {
                    Path::new(path)
                        .file_name()
                        .is_some_and(|name| name == basename.as_ref())
                })
                .collect();
            if matches.len() == 1 {
                return matches.first().copied().cloned();
            }
        }
    }

    None
}

/// Returns whether `target` already names a markdown file.
fn has_markdown_extension(target: &str) -> bool {
    Path::new(target)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// Builds the ordered resolution candidates for `reference` relative to
/// `source_path`.
fn reference_candidates(reference: &str, source_path: &Path) -> Vec<String> {
    let mut candidates = Vec::new();
    candidates.push(reference.to_owned());
    if let Some(parent) = source_path.parent() {
        let joined = parent.join(reference);
        candidates.push(joined.to_string_lossy().into_owned());
    }
    candidates
}

/// Extracts wikilink path targets from markdown `content`.
///
/// Recognizes `[[target]]`, `[[target|label]]`, `[[path/target#heading]]`, and
/// `[[target#heading|label]]`: the piped label and the header fragment are
/// stripped, and bare self-anchors (`[[#heading]]`), empty targets, and
/// `http(s)` targets are dropped. A wikilink is closed by the first `]]`
/// (ADR-014).
fn inline_wikilinks(content: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(content);
    let mut targets = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;

    while let Some(pos) = find_subslice(bytes, b"[[", index) {
        let after = pos + 2;
        let Some(rest) = bytes.get(after..) else {
            break;
        };
        let Some(relative) = find_subslice(rest, b"]]", 0) else {
            break;
        };
        let end = after + relative;
        let raw = text.get(after..end).unwrap_or_default();
        let without_label = raw.split_once('|').map_or(raw, |(path, _)| path);
        let without_fragment = without_label
            .split_once('#')
            .map_or(without_label, |(path, _)| path);
        let target = without_fragment.trim();
        if !target.is_empty() && !target.starts_with("http://") && !target.starts_with("https://") {
            targets.push(target.to_owned());
        }
        index = end + 2;
    }

    targets
}

/// Resolves a wikilink target to a known file path within the collection.
///
/// Resolution mirrors [`resolve_file`]'s candidate strategy (target as-is,
/// joined onto the source file's directory, `.md`-appended variants, then a
/// basename match) but matches known paths case-insensitively, Obsidian-style.
/// A candidate with more than one case-insensitive match is ambiguous and
/// skipped; unresolved targets yield `None` (ADR-014).
fn resolve_wikilink(target: &str, source_path: &Path, known: &HashSet<String>) -> Option<String> {
    let target = target.trim();
    if target.is_empty() || target.starts_with("http://") || target.starts_with("https://") {
        return None;
    }

    let mut candidates = reference_candidates(target, source_path);
    if !has_markdown_extension(target) {
        let appended = format!("{target}.md");
        candidates.push(appended.clone());
        if let Some(parent) = source_path.parent() {
            candidates.push(parent.join(&appended).to_string_lossy().into_owned());
        }
    }

    for candidate in &candidates {
        if let Some(resolved) = exact_case_insensitive(candidate, known) {
            return Some(resolved);
        }
    }

    let mut basenames = vec![Path::new(target).to_path_buf()];
    if !has_markdown_extension(target) {
        basenames.push(Path::new(&format!("{target}.md")).to_path_buf());
    }
    for basename in basenames {
        if let Some(basename) = basename.file_name() {
            let basename = basename.to_string_lossy();
            let mut matches = known.iter().filter(|path| {
                Path::new(path)
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(&basename))
            });
            let Some(first) = matches.next() else {
                continue;
            };
            if matches.next().is_some() {
                continue;
            }
            return Some(first.clone());
        }
    }

    None
}

/// Returns the single known path equal to `candidate` ignoring ASCII case, or
/// `None` when none or more than one match exists.
fn exact_case_insensitive(candidate: &str, known: &HashSet<String>) -> Option<String> {
    let mut matches = known
        .iter()
        .filter(|path| path.eq_ignore_ascii_case(candidate));
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(first.clone())
}

/// Extracts inline relative `.md` link targets from markdown `content`.
///
/// Only targets that name a `.md` file and are not bare anchors are returned.
fn inline_markdown_links(content: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(content);
    let mut targets = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;

    while let Some(pos) = find_subslice(bytes, b"](", index) {
        let after = pos + 2;
        let Some(rest) = bytes.get(after..) else {
            break;
        };
        let end = match rest.iter().position(|&byte| byte == b')') {
            Some(relative) => after + relative,
            None => break,
        };
        let raw = text.get(after..end).unwrap_or_default();
        let target = strip_link_target(raw);
        if is_markdown_target(&target) {
            targets.push(target);
        }
        index = end + 1;
    }

    targets
}

/// Strips angle brackets and an anchor fragment from a raw link target.
fn strip_link_target(raw: &str) -> String {
    let trimmed = raw.trim();
    let inner =
        if let (Some(stripped), Some(_)) = (trimmed.strip_prefix('<'), trimmed.strip_suffix('>')) {
            stripped
        } else {
            trimmed
        };
    match inner.split_once('#') {
        Some((path, _)) => path.to_owned(),
        None => inner.to_owned(),
    }
}

/// Returns whether `target` names a markdown file.
fn is_markdown_target(target: &str) -> bool {
    let trimmed = target.trim();
    if trimmed.is_empty() || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return false;
    }
    PathBuf::from(trimmed)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// Finds the first occurrence of `needle` in `haystack` starting at `from`.
fn find_subslice(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > haystack.len() {
        return None;
    }
    (from..=(haystack.len() - needle.len()))
        .find(|&start| haystack.get(start..start + needle.len()) == Some(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileId;

    fn source(
        id: u64,
        path: &'static str,
        content: &'static str,
    ) -> Result<GraphSource<'static>, crate::FileIdError> {
        Ok(GraphSource::new(
            FileId::try_new(id)?,
            Path::new(path),
            content.as_bytes(),
        ))
    }

    fn node(graph: &EntityGraph, kind: EntityKind, key: &str) -> bool {
        graph
            .nodes()
            .any(|n| n.id().kind() == kind && n.id().key() == key)
    }

    fn edge(graph: &EntityGraph, from: &str, relation: RelationKind, to: &str) -> bool {
        graph
            .edges()
            .any(|e| e.src().key() == from && e.relation() == relation && e.dst().key() == to)
    }

    #[test]
    fn file_and_tag_nodes_with_edges() -> Result<(), Box<dyn std::error::Error>> {
        let files = [
            source(1, "a.md", "---\ntags: [rust]\n---\n[link](b.md)\n")?,
            source(2, "b.md", "---\ntags: [rust]\n---\nbody\n")?,
        ];
        let graph = extract_graph(&files);

        assert!(node(&graph, EntityKind::File, "a.md"));
        assert!(node(&graph, EntityKind::File, "b.md"));
        assert!(node(&graph, EntityKind::Tag, "rust"));
        assert!(edge(&graph, "a.md", RelationKind::LinksTo, "b.md"));
        assert!(edge(&graph, "a.md", RelationKind::TaggedWith, "rust"));
        assert!(edge(&graph, "b.md", RelationKind::TaggedWith, "rust"));

        Ok(())
    }

    #[test]
    fn alias_nodes_and_edges() -> Result<(), Box<dyn std::error::Error>> {
        let files = [source(1, "a.md", "---\naliases: [mt, my]\n---\nbody\n")?];
        let graph = extract_graph(&files);

        assert!(node(&graph, EntityKind::Alias, "mt"));
        assert!(node(&graph, EntityKind::Alias, "my"));
        assert!(edge(&graph, "a.md", RelationKind::AliasOf, "mt"));
        assert!(edge(&graph, "a.md", RelationKind::AliasOf, "my"));

        Ok(())
    }

    #[test]
    fn related_and_sources_edges() -> Result<(), Box<dyn std::error::Error>> {
        let files = [
            source(1, "a.md", "---\nrelated: [b]\nsources: [c]\n---\nbody\n")?,
            source(2, "b.md", "---\n---\nbody\n")?,
            source(3, "c.md", "---\n---\nbody\n")?,
        ];
        let graph = extract_graph(&files);

        assert!(edge(&graph, "a.md", RelationKind::RelatedTo, "b.md"));
        assert!(edge(&graph, "a.md", RelationKind::HasSource, "c.md"));

        Ok(())
    }

    #[test]
    fn unresolved_related_skipped() -> Result<(), Box<dyn std::error::Error>> {
        let files = [source(1, "a.md", "---\nrelated: [missing]\n---\nbody\n")?];
        let graph = extract_graph(&files);

        assert!(!graph.edges().any(|e| e.dst().key() == "missing"));

        Ok(())
    }

    #[test]
    fn tag_and_alias_collision_distinct() -> Result<(), Box<dyn std::error::Error>> {
        let files = [source(
            1,
            "a.md",
            "---\ntags: [mt]\naliases: [mt]\n---\nbody\n",
        )?];
        let graph = extract_graph(&files);

        let tag = graph
            .nodes()
            .filter(|n| n.id().kind() == EntityKind::Tag && n.id().key() == "mt")
            .count();
        let alias = graph
            .nodes()
            .filter(|n| n.id().kind() == EntityKind::Alias && n.id().key() == "mt")
            .count();
        assert_eq!(tag, 1);
        assert_eq!(alias, 1);

        Ok(())
    }

    #[test]
    fn empty_collection_is_empty_graph() {
        let graph = extract_graph(&[]);
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn extraction_is_deterministic_and_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let files = [
            source(1, "a.md", "---\ntags: [rust]\n---\n[to](b.md)\n")?,
            source(2, "b.md", "---\n---\nbody\n")?,
        ];
        let first = extract_graph(&files);
        let second = extract_graph(&files);
        assert_eq!(first, second);
        assert_eq!(first.node_count(), 3);
        assert_eq!(first.edge_count(), 2);

        Ok(())
    }

    #[test]
    fn duplicate_edges_deduplicated() -> Result<(), Box<dyn std::error::Error>> {
        let files = [
            source(1, "a.md", "---\ntags: [rust, rust]\n---\nbody\n")?,
            source(2, "b.md", "---\n---\nbody\n")?,
        ];
        let graph = extract_graph(&files);
        let tagged = graph
            .edges()
            .filter(|e| e.relation() == RelationKind::TaggedWith)
            .count();
        assert_eq!(tagged, 1);

        Ok(())
    }

    /// Covers: REQ-019 FR-001/FR-002/FR-003 — wikilink forms produce
    /// `LINKS_TO` edges; labels are ignored; fragments are stripped.
    #[test]
    fn wikilink_forms_produce_links_to_edges() -> Result<(), Box<dyn std::error::Error>> {
        let files = [
            source(
                1,
                "a.md",
                "[[borrowing]] [[notes/borrowing|Borrowing Rules]] [[borrowing#Lifetimes]] [[borrowing#Lifetimes|L]]\n",
            )?,
            source(2, "borrowing.md", "body\n")?,
            source(3, "notes/borrowing.md", "body\n")?,
        ];
        let graph = extract_graph(&files);

        assert!(edge(&graph, "a.md", RelationKind::LinksTo, "borrowing.md"));
        assert!(edge(
            &graph,
            "a.md",
            RelationKind::LinksTo,
            "notes/borrowing.md"
        ));
        assert!(!node(&graph, EntityKind::Alias, "Borrowing Rules"));

        Ok(())
    }

    /// Covers: REQ-019 FR-003/FR-009 — wikilink extraction strips labels and
    /// fragments and drops empty, self-anchor, and http targets.
    #[test]
    fn wikilink_extraction_strips_and_filters() {
        let content = "[[a]] [[b|c]] [[d#e]] [[f#g|h]] [[#x]] [[]] [[http://x]] [[https://y]]";

        assert_eq!(
            inline_wikilinks(content.as_bytes()),
            vec!["a", "b", "d", "f"]
        );
    }

    /// Covers: REQ-019 FR-005 — wikilink resolution is case-insensitive.
    #[test]
    fn wikilink_resolution_is_case_insensitive() {
        let known = HashSet::from(["note.md".to_owned()]);

        assert_eq!(
            resolve_wikilink("Note", Path::new("a.md"), &known),
            Some("note.md".to_owned())
        );
    }

    /// Covers: REQ-019 FR-005 — an ambiguous case-only match is skipped.
    #[test]
    fn ambiguous_case_only_match_is_skipped() {
        let known = HashSet::from(["Note.md".to_owned(), "note.md".to_owned()]);

        assert_eq!(resolve_wikilink("Note", Path::new("a.md"), &known), None);
    }

    /// Covers: REQ-019 FR-006 — an unresolved wikilink produces no edge.
    #[test]
    fn unresolved_wikilink_produces_no_edge() -> Result<(), Box<dyn std::error::Error>> {
        let files = [source(1, "a.md", "[[missing]]\n")?];
        let graph = extract_graph(&files);

        assert_eq!(graph.edge_count(), 0);

        Ok(())
    }

    /// Covers: REQ-019 FR-004/FR-007 — self-anchors and self-links produce no
    /// edges while links from other files still resolve.
    #[test]
    fn self_wikilinks_produce_no_edges() -> Result<(), Box<dyn std::error::Error>> {
        let files = [
            source(1, "note.md", "[[note]] [[#Overview]]\n")?,
            source(2, "a.md", "[[note]]\n")?,
        ];
        let graph = extract_graph(&files);

        assert!(!edge(&graph, "note.md", RelationKind::LinksTo, "note.md"));
        assert!(edge(&graph, "a.md", RelationKind::LinksTo, "note.md"));

        Ok(())
    }

    /// Covers: REQ-019 FR-008 — markdown links and wikilinks coexist.
    #[test]
    fn markdown_links_and_wikilinks_coexist() -> Result<(), Box<dyn std::error::Error>> {
        let files = [
            source(1, "a.md", "[label](target.md) [[other]]\n")?,
            source(2, "target.md", "body\n")?,
            source(3, "other.md", "body\n")?,
        ];
        let graph = extract_graph(&files);

        assert!(edge(&graph, "a.md", RelationKind::LinksTo, "target.md"));
        assert!(edge(&graph, "a.md", RelationKind::LinksTo, "other.md"));

        Ok(())
    }
}
