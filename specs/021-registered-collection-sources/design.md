---
id: DES-021
title: "Persist collection sources and reconcile source-driven updates"
type: feature-design
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin explicitly approved DES-021 and its schema proposal in chat on 2026-09-30"
parent: US-021
depends_on: []
requires:
  - DB-001
  - TABLE-013
blockers: []
related:
  - REQ-021
  - US-005
  - US-006
  - US-012
  - US-016
  - ADR-001
  - ADR-005
  - ADR-015
  - DB-001
  - TABLE-013
---

# Design

## Context And Constraints

Collections currently persist files and indexes but not their filesystem
sources. `collection update` therefore needs paths supplied again, and update-all
cannot discover additions. The app already reconciles files and rebuilds lexical
and graph indexes in a single SQLite transaction. This slice adds persistent
source registration and uses that existing transactional boundary.

The implementation stays in the current Rust layers and SQLite adapter. It adds
no crate, workspace member, external service, or dependency. Existing databases
must retain their contents; source paths are never inferred from stored files.
New table schemas use the `id` primary-key rule in `R-DB-01`.

## Proposed Design

Add a normalized `collection_sources` table, documented by `TABLE-013`, with a
database-generated `id`, collection foreign key, canonical absolute source
path, and source kind (`file` or `directory`). A unique constraint on
`(collection_id, source_path)` prevents duplicate registrations. The kind is
stored because a file source that disappears is a deletion, while an
inaccessible directory must abort its collection update. Database schema
version 8 creates the table; existing rows remain source-less and searchable.

Extend the existing collection persistence boundary to create a collection and
its initial sources atomically, replace a collection's source set atomically,
and list collection/source summaries. Add a source value type in `domain` to
carry the canonical path and kind without filesystem behavior. The filesystem
port resolves and validates paths during registration and expands a stored
source during update. No filesystem policy moves into SQLite or the domain.

The source-aware update loads the selected collection's source set, expands
registered directories recursively, includes registered `.md` file sources,
and canonicalizes/deduplicates discovered file paths before reading content.
Directory enumeration failures abort that collection regardless of
`--skip-unreadable`; individual file read failures follow that switch and
preserve any prior stored content. The update compares the discovered file set
with stored files, then sends all upserts and deletions to the existing
`FileStore::reconcile` operation. That operation already rebuilds lexical
passages and graph nodes/edges within the same SQLite transaction as the file
changes. A failure rolls back all three. Removing a source therefore removes
only files absent from every remaining source.

Update-all enumerates collections and runs this per-collection flow
independently. It collects each success or failure, continues after failures,
and returns a failed overall result when one or more collections fail. It does
not wrap all collections in one database transaction.

Collection listing returns stable, case-insensitively ordered collection
summaries and source paths ordered by canonical path. Human output names each
collection and its sources. JSON uses the approved `collections` array with
`name` and `sources` fields. Collection destruction removes source records
alongside other collection-owned data, preserving US-016's no-orphan invariant.

## Components And Responsibilities

| Component | Responsibility | Depends on |
| --- | --- | --- |
| `domain` source value | Represent a validated canonical source path and file/directory kind | `std::path` only |
| `application` collection persistence port | Create with sources, replace sources, and list collection/source summaries | Domain types |
| `application::UpdateCollection` | Load sources, request filesystem discovery, classify changes, and call reconcile once | Filesystem, collection persistence, file store, clock ports |
| `application` collection use cases | Expose create/configure/list behavior without doing I/O directly | Collection persistence and filesystem ports |
| Filesystem adapter | Canonicalize registration paths, recursively expand directory roots, read files, and report typed failures | Existing standard-library filesystem integration |
| SQLite collection/file stores | Persist source rows and reconcile files/indexes transactionally | SQLite adapter |
| CLI app | Parse source/update/list commands, render summaries and update-all reports, map failures to process exit status | Existing app composition root |

## Interfaces And Contracts

| Interface | Inputs | Outputs | Errors |
| --- | --- | --- | --- |
| Create collection with sources | Collection name, source paths, timestamp | Stored collection and canonical source set | Duplicate/invalid collection, invalid source, storage failure; no partial collection/source rows |
| Configure sources | Collection name and replacement paths | Persisted canonical source set | Missing collection, invalid source, storage failure; previous set remains on failure |
| List collections with sources | None | Ordered summaries with source paths | Database or storage failure |
| Source-aware update | Collection name and skip-unreadable option | Added/modified/deleted/skipped and malformed-frontmatter counts | Missing collection/source set, source enumeration, file read, clock, or reconcile failure |
| Update all | Skip-unreadable option | Per-collection outcomes and aggregate success/failure | Continues through individual collection errors; aggregate is failed if any fail |

## Data And State Flow

```mermaid
sequenceDiagram
    participant CLI
    participant App as Application
    participant FS as Filesystem Port
    participant DB as SQLite Adapter

    CLI->>App: create/configure collection sources
    App->>FS: canonicalize and classify paths
    FS-->>App: canonical source values
    App->>DB: persist collection/source set in transaction
    DB-->>App: success
    App-->>CLI: confirmation

    CLI->>App: update collection / update all
    App->>DB: load registered sources and stored files
    DB-->>App: source set and file summaries
    App->>FS: expand roots and read discovered files
    FS-->>App: unique canonical records or typed error
    App->>DB: reconcile upserts/deletions and rebuild lexical + graph
    Note over DB: One collection transaction
    DB-->>App: committed outcome or rollback error
    App-->>CLI: outcome; update-all continues per collection
```

Registration validates and canonicalizes before changing persisted state. A
create-with-sources operation writes collection and source rows in one
transaction. Configure replaces all source rows in one transaction. Listing
reads collection names and their ordered source paths.

During update, an inaccessible directory fails before persistence starts. An
unreadable individual file with skip enabled is excluded from deletion
classification, preserving an existing database row. Otherwise, the app
classifies the complete discovery against stored content hashes and submits
upserts/deletions to `reconcile`. The SQLite adapter applies file changes,
rebuilds lexical and graph state, and commits once. If any write/build/commit
fails, SQLite rolls back the collection update. Update-all records that error
and proceeds to the next collection.

Migration adds the source table at version 8 without populating it. A collection
with no source rows is treated as legacy/unconfigured: reads and searches remain
available, while update reports that the user must register sources. Destroy
cleanup includes the new source table so removed collections leave no source
records.

## Security, Performance, And Operations

- Security: Source paths remain local filesystem paths. The CLI performs no network access. Directory enumeration failures are surfaced and never interpreted as a successful empty scan.
- Performance: Each update scans registered directories and deduplicates canonical `.md` paths. Index rebuilding remains a full collection rebuild as established by ADR-005.
- Operations: Migration is additive and preserves all legacy content. A failed collection update is retryable because the previous content and indexes remain intact. Update-all reports partial success without implying database-wide atomicity.

## Alternatives Considered

| Alternative | Why not chosen |
| --- | --- |
| Infer source roots by taking common parents of stored file paths | Ambiguous and can expand update scope beyond the user's intent; migration must not guess |
| Store roots as one serialized field on `collections` | Couples source-list mutation/uniqueness to collection rows and gives up relational uniqueness and typed source kind |
| Keep requiring explicit paths for every update | Does not let update-all discover new files and fails the approved workflow |
| Use one transaction for all collections in update-all | A single collection failure would roll back unrelated work and conflict with the approved per-collection continuation behavior |
| Rebuild semantic indexes in this slice | Semantic enablement and combined semantic atomicity belong to the later semantic configuration slice |

## Risks And Open Decisions

- The source kind must be captured at registration so later disappearance has deterministic file-versus-directory behavior.
- A source list may change after discovery but before reconcile in another CLI process. The CLI remains an explicit local tool; updates use the source snapshot read at start, and a later update observes subsequent configuration.
- Existing semantic indexes may become stale when file contents change. Semantic freshness/rebuild behavior is handled by the subsequent semantic-index slice; this design does not claim to refresh semantic vectors.
- No new ADR is needed: SQLite persistence follows ADR-001, lexical/index transaction behavior follows ADR-005, and the source table is specified in `TABLE-013`.

## Verification Approach

- Application tests use fake filesystem, collection, and file-store ports to cover source expansion, path deduplication, deletion classification, unreadable-file preservation, and update-all continuation.
- SQLite adapter tests cover version-8 migration from versions 0 through 7, source create/replace/list behavior, uniqueness, and collection-destroy cleanup.
- Adapter transaction tests inject lexical and graph build failures and prove stored files and both indexes retain their prior state.
- CLI subprocess tests cover registration without indexing, listing in human/JSON formats, new-file discovery after process restart, legacy source-less collections, and update-all's aggregate exit status.
- Run `cargo xtask ci` and `cargo xtask eval`; source changes do not alter query semantics, so evaluation must preserve its existing baseline.
