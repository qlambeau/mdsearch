use std::io::{self, Write};

use indicatif::{ProgressBar, ProgressDrawTarget};
use kv_application::EmbedProgress;
use kv_domain::CollectionName;

/// Renders `EmbedProgress` events to stderr during an embedding run.
///
/// The renderer keeps one `indicatif` progress bar per collection: a `Files`
/// event starts (or advances) the bar for its collection, and a `Writing`
/// event finalizes it and prints a single status line. Rendering is
/// best-effort: a stderr write failure never fails the run (REQ-018 FR-008).
#[derive(Default)]
pub(crate) struct ProgressRenderer {
    bar: Option<ProgressBar>,
    collection: Option<CollectionName>,
}

impl ProgressRenderer {
    /// Handles one progress event.
    pub(crate) fn handle(&mut self, event: EmbedProgress) {
        match event {
            EmbedProgress::Files {
                collection,
                completed_files,
                total_files,
            } => {
                if self.collection.as_ref() != Some(&collection) {
                    self.finish_bar();
                    self.collection = Some(collection.clone());
                    self.bar = Some(ProgressBar::with_draw_target(
                        Some(total_files as u64),
                        ProgressDrawTarget::stderr(),
                    ));
                }
                if let Some(bar) = &self.bar {
                    bar.set_message(format!("embedding {}", collection.display_name()));
                    bar.set_position(completed_files as u64);
                }
            }
            EmbedProgress::Writing { collection } => {
                self.finish_bar();
                let mut stderr = io::stderr().lock();
                let _ = writeln!(
                    stderr,
                    "writing semantic index for {}...",
                    collection.display_name()
                );
            }
        }
    }

    /// Finalizes any in-flight progress bar, leaving a clean stderr state.
    pub(crate) fn finish(&mut self) {
        self.finish_bar();
    }

    fn finish_bar(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
        self.collection = None;
    }
}
