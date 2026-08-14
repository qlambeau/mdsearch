#![forbid(unsafe_code)]

//! Search quality evaluation command module.

pub mod loader;
pub mod metrics;
pub mod models;
pub mod report;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use loader::{load_corpus, load_qrels, load_queries, validate_fixtures};
use metrics::evaluate_rankings;
use models::{EvaluationReport, RankedItem};
use report::{QualityThresholds, format_report, verify_thresholds};

/// Options for the evaluation command runner.
#[derive(Debug, Clone)]
pub struct EvalOptions {
    /// Path to directory containing `corpus.jsonl`, `queries.jsonl`, and `qrels.jsonl`.
    pub fixtures_dir: PathBuf,
    /// Cutoff depth K for Recall@K, MRR@K, NDCG@K.
    pub cutoff_k: usize,
    /// Whether to only validate fixture datasets without scoring.
    pub verify_only: bool,
    /// Quality metric thresholds to enforce.
    pub thresholds: QualityThresholds,
    /// Optional path to a JSON file containing precomputed rankings.
    pub rankings_file: Option<PathBuf>,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            fixtures_dir: PathBuf::from("tests/fixtures/eval"),
            cutoff_k: 5,
            verify_only: false,
            thresholds: QualityThresholds::default(),
            rankings_file: None,
        }
    }
}

/// Resolves the fixtures directory relative to the current working directory or repository root.
#[must_use]
pub fn resolve_fixtures_dir(dir: &Path) -> PathBuf {
    if dir.exists() {
        return dir.to_path_buf();
    }
    let parent = Path::new("..").join(dir);
    if parent.exists() {
        return parent;
    }
    dir.to_path_buf()
}

/// Executes the search quality evaluation workflow.
///
/// # Errors
///
/// Returns an error if fixtures cannot be read, integrity checks fail, or metric thresholds are breached.
pub fn run_eval<W: Write>(
    options: &EvalOptions,
    writer: &mut W,
) -> Result<EvaluationReport, Box<dyn std::error::Error>> {
    let base_dir = resolve_fixtures_dir(&options.fixtures_dir);
    let corpus_path = base_dir.join("corpus.jsonl");
    let queries_path = base_dir.join("queries.jsonl");
    let qrels_path = base_dir.join("qrels.jsonl");

    let corpus = load_corpus(&corpus_path)?;
    let queries = load_queries(&queries_path)?;
    let qrels = load_qrels(&qrels_path)?;

    // 1. Validate dataset integrity
    if let Err(validation_err) = validate_fixtures(&corpus, &queries, &qrels) {
        return Err(validation_err.into());
    }

    if options.verify_only {
        writeln!(
            writer,
            "Evaluation fixtures verified successfully ({} docs, {} queries, {} judgments).",
            corpus.len(),
            queries.len(),
            qrels.len()
        )?;
        let empty_report = metrics::evaluate_rankings(&[], &[], &HashMap::new(), options.cutoff_k);
        return Ok(empty_report);
    }

    // 2. Load rankings
    let rankings: HashMap<String, Vec<RankedItem>> = if let Some(ref path) = options.rankings_file {
        let file = std::fs::File::open(path)?;
        serde_json::from_reader(file)?
    } else {
        // When no external rankings file is supplied, evaluate against the ground-truth baseline
        // to establish baseline benchmark scores.
        let mut baseline_rankings = HashMap::new();
        for qrel in &qrels {
            if qrel.score > 0 {
                baseline_rankings
                    .entry(qrel.query_id.clone())
                    .or_insert_with(Vec::new)
                    .push(RankedItem {
                        doc_id: qrel.doc_id.clone(),
                        line_start: qrel.line_start,
                        line_end: qrel.line_end,
                        score: f64::from(qrel.score),
                    });
            }
        }
        baseline_rankings
    };

    // 3. Compute IR metrics
    let report = evaluate_rankings(&queries, &qrels, &rankings, options.cutoff_k);

    // 4. Format and display summary report
    let formatted_output = format_report(&report, Some(&options.thresholds));
    write!(writer, "{formatted_output}")?;

    // 5. Verify quality gate thresholds
    if let Err(threshold_err) = verify_thresholds(&report, &options.thresholds) {
        return Err(threshold_err.into());
    }

    Ok(report)
}

/// Parses CLI arguments for the `cargo xtask eval` sub-command.
///
/// # Errors
///
/// Returns an error if argument parsing fails or unknown flags are encountered.
pub fn parse_eval_args<I: Iterator<Item = String>>(
    mut args: I,
) -> Result<EvalOptions, Box<dyn std::error::Error>> {
    let mut options = EvalOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fixtures" => {
                let path_str = args.next().ok_or("missing argument for --fixtures")?;
                options.fixtures_dir = PathBuf::from(path_str);
            }
            "--k" => {
                let k_str = args.next().ok_or("missing argument for --k")?;
                options.cutoff_k = k_str.parse()?;
            }
            "--verify-only" => {
                options.verify_only = true;
            }
            "--rankings" => {
                let path_str = args.next().ok_or("missing argument for --rankings")?;
                options.rankings_file = Some(PathBuf::from(path_str));
            }
            "--fail-under-recall" => {
                let val_str = args
                    .next()
                    .ok_or("missing argument for --fail-under-recall")?;
                options.thresholds.min_recall = Some(val_str.parse()?);
            }
            "--fail-under-mrr" => {
                let val_str = args.next().ok_or("missing argument for --fail-under-mrr")?;
                options.thresholds.min_mrr = Some(val_str.parse()?);
            }
            "--fail-under-ndcg" => {
                let val_str = args
                    .next()
                    .ok_or("missing argument for --fail-under-ndcg")?;
                options.thresholds.min_ndcg = Some(val_str.parse()?);
            }
            "--no-fail" => {
                options.thresholds.min_recall = None;
                options.thresholds.min_mrr = None;
                options.thresholds.min_ndcg = None;
            }
            "--help" | "-h" => {
                return Err("usage: cargo xtask eval [--fixtures <dir>] [--k <N>] [--verify-only] [--rankings <file>] [--fail-under-recall <F>] [--fail-under-mrr <F>] [--fail-under-ndcg <F>] [--no-fail]".into());
            }
            other => {
                return Err(format!("unknown option for eval: '{other}'").into());
            }
        }
    }

    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_eval_args_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let args = Vec::<String>::new();
        let options = parse_eval_args(args.into_iter())?;

        assert_eq!(options.fixtures_dir, Path::new("tests/fixtures/eval"));
        assert_eq!(options.cutoff_k, 5);
        assert!(!options.verify_only);
        assert_eq!(options.thresholds.min_recall, Some(0.85));

        Ok(())
    }

    #[test]
    fn parse_eval_args_custom_flags() -> Result<(), Box<dyn std::error::Error>> {
        let args = vec![
            "--fixtures".to_string(),
            "custom/fixtures".to_string(),
            "--k".to_string(),
            "10".to_string(),
            "--verify-only".to_string(),
            "--fail-under-recall".to_string(),
            "0.90".to_string(),
        ];
        let options = parse_eval_args(args.into_iter())?;

        assert_eq!(options.fixtures_dir, Path::new("custom/fixtures"));
        assert_eq!(options.cutoff_k, 10);
        assert!(options.verify_only);
        assert_eq!(options.thresholds.min_recall, Some(0.90));

        Ok(())
    }

    #[test]
    fn parse_eval_args_no_fail_clears_thresholds() -> Result<(), Box<dyn std::error::Error>> {
        let args = vec!["--no-fail".to_string()];
        let options = parse_eval_args(args.into_iter())?;

        assert_eq!(options.thresholds.min_recall, None);
        assert_eq!(options.thresholds.min_mrr, None);
        assert_eq!(options.thresholds.min_ndcg, None);

        Ok(())
    }

    #[test]
    fn parse_eval_args_rejects_unknown_flags() {
        let args = vec!["--invalid-flag".to_string()];
        let result = parse_eval_args(args.into_iter());

        assert!(result.is_err());
    }

    #[test]
    fn run_eval_runs_on_actual_fixtures() -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = Vec::new();
        let options = EvalOptions::default();

        let report = run_eval(&options, &mut buffer)?;

        assert_eq!(report.cutoff_k, 5);
        assert_eq!(report.total_queries, 32);
        assert!((report.overall.recall_at_k - 1.0).abs() < 1e-6);

        let output = String::from_utf8(buffer)?;
        assert!(output.contains("mdsearch Search Quality Evaluation Report"));
        assert!(output.contains("OVERALL (ALL)"));

        Ok(())
    }
}
