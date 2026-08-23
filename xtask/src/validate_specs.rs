#![forbid(unsafe_code)]

//! Specification validator for Spec-Driven Development (SDD) artifacts.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Summary report of specification validation.
#[derive(Debug, Default)]
pub struct ValidationReport {
    /// Total number of markdown specification artifacts scanned.
    pub total_artifacts: usize,
    /// List of fatal validation errors found.
    pub errors: Vec<String>,
    /// List of non-fatal warnings found.
    pub warnings: Vec<String>,
}

/// Structured metadata extracted from an artifact's YAML frontmatter.
#[derive(Debug, Clone)]
pub struct ParsedArtifact {
    /// Filesystem path to the artifact.
    pub path: PathBuf,
    /// Formal specification ID (e.g. US-001, ADR-005).
    pub id: String,
    /// Document title.
    pub title: String,
    /// Artifact type tag.
    pub artifact_type: String,
    /// Lifecycle status.
    pub status: String,
    /// Artifact owner.
    pub owner: Option<String>,
    /// Parent artifact reference.
    pub parent: Option<String>,
    /// Target database ID (for table schemas).
    pub database: Option<String>,
    /// List of table IDs (for database schemas).
    pub tables: Vec<String>,
}

/// Resolves the specs directory relative to the current working directory or repository root.
#[must_use]
pub fn resolve_specs_dir() -> Option<PathBuf> {
    let dir = Path::new("specs");
    if dir.exists() {
        return Some(dir.to_path_buf());
    }
    let parent = Path::new("../specs");
    if parent.exists() {
        return Some(parent.to_path_buf());
    }
    None
}

/// Runs the specification validation check.
///
/// # Errors
/// Returns an error if directory traversal fails or if validation errors are detected.
pub fn run_validate_specs(out: &mut dyn Write) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(out, "==> Validating SDD Specifications in specs/...")?;

    let specs_dir = resolve_specs_dir().ok_or("specs directory not found")?;

    let mut report = ValidationReport::default();
    let mut artifacts: HashMap<String, ParsedArtifact> = HashMap::new();

    // 1. Scan and parse all specs (skipping templates and archive)
    scan_directory(&specs_dir, &mut artifacts, &mut report)?;
    report.total_artifacts = artifacts.len();

    // 2. Validate database / table bidirectional references
    validate_schema_references(&artifacts, &mut report);

    // 3. Validate feature packet integrity
    validate_feature_packets(&specs_dir, &artifacts, &mut report)?;

    // 4. Output results
    writeln!(
        out,
        "Scanned {} specification artifacts.",
        report.total_artifacts
    )?;

    if !report.warnings.is_empty() {
        writeln!(out, "\n[WARNINGS] ({}):", report.warnings.len())?;
        for warning in &report.warnings {
            writeln!(out, "  - {warning}")?;
        }
    }

    if !report.errors.is_empty() {
        writeln!(out, "\n[ERRORS] ({}):", report.errors.len())?;
        for error in &report.errors {
            writeln!(out, "  - {error}")?;
        }
        return Err(format!(
            "Specification validation failed with {} errors.",
            report.errors.len()
        )
        .into());
    }

    writeln!(
        out,
        "\nAll specification validation checks passed successfully!"
    )?;
    Ok(())
}

fn scan_directory(
    dir: &Path,
    artifacts: &mut HashMap<String, ParsedArtifact>,
    report: &mut ValidationReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if file_name == "templates" || file_name == "archive" || file_name.starts_with('.') {
                continue;
            }
            scan_directory(&path, artifacts, report)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            process_md_file(&path, artifacts, report);
        }
    }
    Ok(())
}

fn process_md_file(
    path: &Path,
    artifacts: &mut HashMap<String, ParsedArtifact>,
    report: &mut ValidationReport,
) {
    if let Some(artifact) = parse_markdown_artifact(path, report) {
        if let Some(existing) = artifacts.get(&artifact.id) {
            report.errors.push(format!(
                "ID collision detected: '{}' is used in both '{}' and '{}'",
                artifact.id,
                existing.path.display(),
                artifact.path.display()
            ));
        } else {
            artifacts.insert(artifact.id.clone(), artifact);
        }
    }
}

#[derive(Default)]
struct RawFrontmatter {
    id: Option<String>,
    title: Option<String>,
    artifact_type: Option<String>,
    status: Option<String>,
    owner: Option<String>,
    parent: Option<String>,
    database: Option<String>,
    tables: Vec<String>,
}

fn extract_frontmatter(content: &str) -> Option<RawFrontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let mut raw = RawFrontmatter::default();
    let mut in_tables = false;

    for line in content.lines().skip(1) {
        if line.trim() == "---" {
            break;
        }

        if in_tables {
            if line.trim().starts_with("- ") {
                let table_id = line.trim()[2..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                raw.tables.push(table_id);
                continue;
            } else if !line.starts_with(' ') && !line.starts_with('\t') {
                in_tables = false;
            }
        }

        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim();
            let val = val.trim().trim_matches('"').trim_matches('\'');

            match key {
                "id" => raw.id = Some(val.to_string()),
                "title" => raw.title = Some(val.to_string()),
                "type" => raw.artifact_type = Some(val.to_string()),
                "status" => raw.status = Some(val.to_string()),
                "owner" => raw.owner = Some(val.to_string()),
                "parent" if val != "null" && !val.is_empty() => {
                    raw.parent = Some(val.to_string());
                }
                "database" => raw.database = Some(val.to_string()),
                "tables" => {
                    in_tables = true;
                }
                _ => {}
            }
        }
    }

    Some(raw)
}

fn parse_markdown_artifact(path: &Path, report: &mut ValidationReport) -> Option<ParsedArtifact> {
    let content = fs::read_to_string(path).ok()?;
    let raw = extract_frontmatter(&content)?;

    let id = raw.id?;
    validate_id_format(&id, path, report);

    let title = raw.title.unwrap_or_else(|| "Untitled".to_string());
    let artifact_type = raw.artifact_type.unwrap_or_else(|| "unknown".to_string());
    let status = raw.status.unwrap_or_else(|| "draft".to_string());

    if raw.owner.as_deref() == Some("TBD") && status != "draft" {
        report.warnings.push(format!(
            "Artifact '{id}' at '{}' has status '{status}' but owner is 'TBD'",
            path.display()
        ));
    }

    Some(ParsedArtifact {
        path: path.to_path_buf(),
        id,
        title,
        artifact_type,
        status,
        owner: raw.owner,
        parent: raw.parent,
        database: raw.database,
        tables: raw.tables,
    })
}

fn validate_id_format(id: &str, path: &Path, report: &mut ValidationReport) {
    let prefixes = [
        "PRD-", "US-", "REQ-", "DES-", "TASK-", "ADR-", "DB-", "TABLE-", "CHART-", "REL-", "OBS-",
    ];
    let is_valid = prefixes.iter().any(|prefix| {
        if let Some(suffix) = id.strip_prefix(prefix) {
            suffix.len() == 3 && suffix.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    });

    if !is_valid {
        report.errors.push(format!(
            "Invalid ID format '{id}' in '{}'. Expected Prefix-NNN (e.g. US-001, ADR-005).",
            path.display()
        ));
    }
}

fn validate_schema_references(
    artifacts: &HashMap<String, ParsedArtifact>,
    report: &mut ValidationReport,
) {
    for (id, artifact) in artifacts {
        if id.starts_with("DB-") {
            validate_database_entry(id, artifact, artifacts, report);
        } else if id.starts_with("TABLE-") {
            validate_table_entry(id, artifact, artifacts, report);
        }
    }
}

fn validate_database_entry(
    db_id: &str,
    artifact: &ParsedArtifact,
    artifacts: &HashMap<String, ParsedArtifact>,
    report: &mut ValidationReport,
) {
    for table_id in &artifact.tables {
        if let Some(table) = artifacts.get(table_id) {
            if table.database.as_deref() != Some(db_id) {
                report.errors.push(format!(
                    "Schema reference mismatch: Database '{db_id}' lists Table '{table_id}', but Table '{table_id}' points to database '{:?}'",
                    table.database
                ));
            }
        } else {
            report.errors.push(format!(
                "Database '{db_id}' references non-existent Table '{table_id}'"
            ));
        }
    }
}

fn validate_table_entry(
    table_id: &str,
    artifact: &ParsedArtifact,
    artifacts: &HashMap<String, ParsedArtifact>,
    report: &mut ValidationReport,
) {
    if let Some(ref db_id) = artifact.database {
        if let Some(db) = artifacts.get(db_id) {
            if !db.tables.contains(&table_id.to_string()) {
                report.errors.push(format!(
                    "Schema reference mismatch: Table '{table_id}' points to Database '{db_id}', but Database '{db_id}' does not list it in tables"
                ));
            }
        } else {
            report.errors.push(format!(
                "Table '{table_id}' references non-existent Database '{db_id}'"
            ));
        }
    } else {
        report.errors.push(format!(
            "Table '{table_id}' at '{}' is missing required 'database' frontmatter field",
            artifact.path.display()
        ));
    }
}

fn validate_feature_packets(
    specs_dir: &Path,
    artifacts: &HashMap<String, ParsedArtifact>,
    report: &mut ValidationReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(specs_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() && name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            validate_single_feature_packet(&path, &name, artifacts, report);
        }
    }
    Ok(())
}

fn validate_single_feature_packet(
    path: &Path,
    name: &str,
    artifacts: &HashMap<String, ParsedArtifact>,
    report: &mut ValidationReport,
) {
    let story_path = path.join("user-story.md");
    let scenarios_path = path.join("scenarios.feature");
    let req_path = path.join("requirements.md");
    let design_path = path.join("design.md");
    let tasks_path = path.join("tasks.md");

    check_file_exists(
        &story_path,
        &format!("Feature packet '{name}' is missing user-story.md"),
        report,
    );
    if scenarios_path.exists() {
        validate_scenarios_file(&scenarios_path, report);
    } else {
        report.errors.push(format!(
            "Feature packet '{name}' is missing scenarios.feature"
        ));
    }
    check_file_exists(
        &req_path,
        &format!("Feature packet '{name}' is missing requirements.md"),
        report,
    );
    check_file_exists(
        &design_path,
        &format!("Feature packet '{name}' is missing design.md"),
        report,
    );
    check_file_exists(
        &tasks_path,
        &format!("Feature packet '{name}' is missing tasks.md"),
        report,
    );

    if let Some(story_artifact) = find_artifact_by_path(artifacts, &story_path) {
        let story_id = &story_artifact.id;
        for check_path in [&req_path, &design_path, &tasks_path] {
            validate_parent_ref(artifacts, check_path, story_id, report);
        }
    }
}

fn validate_parent_ref(
    artifacts: &HashMap<String, ParsedArtifact>,
    check_path: &Path,
    story_id: &str,
    report: &mut ValidationReport,
) {
    if let Some(art) = find_artifact_by_path(artifacts, check_path)
        .filter(|art| art.parent.as_deref() != Some(story_id))
    {
        report.errors.push(format!(
            "Parent mismatch in '{}': expected parent '{story_id}', got '{:?}'",
            check_path.display(),
            art.parent
        ));
    }
}

fn check_file_exists(path: &Path, err_msg: &str, report: &mut ValidationReport) {
    if !path.exists() {
        report.errors.push(err_msg.to_string());
    }
}

fn validate_scenarios_file(path: &Path, report: &mut ValidationReport) {
    if let Ok(content) = fs::read_to_string(path) {
        let has_parent = content.lines().any(|l| l.trim().starts_with("# parent:"));
        let has_status = content.lines().any(|l| l.trim().starts_with("# status:"));

        if !has_parent {
            report.warnings.push(format!(
                "Gherkin scenario file '{}' is missing '# parent: US-NNN' header comment",
                path.display()
            ));
        }
        if !has_status {
            report.warnings.push(format!(
                "Gherkin scenario file '{}' is missing '# status: approved|draft' header comment",
                path.display()
            ));
        }
    }
}

fn find_artifact_by_path<'a>(
    artifacts: &'a HashMap<String, ParsedArtifact>,
    path: &Path,
) -> Option<&'a ParsedArtifact> {
    artifacts.values().find(|a| a.path == path)
}
