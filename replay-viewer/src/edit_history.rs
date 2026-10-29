use crate::replay_log::{ReplayFrame, ReplayLog};

/// Snapshot pair of one edited frame.
#[derive(Clone)]
pub struct FrameChange {
    pub idx: usize,
    pub before: ReplayFrame,
    pub after: ReplayFrame,
}

#[derive(Default)]
pub struct EditHistory {
    undo: Vec<Vec<FrameChange>>,
    redo: Vec<Vec<FrameChange>>,

    /// Snapshot taken when a mouse-hold starts, compared on hold end so a
    /// whole drag stroke (brush/smudge/select-drag) is one undo entry.
    stroke_start: Option<Vec<(usize, ReplayFrame)>>,
}

impl EditHistory {
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.stroke_start = None;
    }

    /// Snapshots `frames[start..end]` (absolute indices) before a stroke.
    pub fn begin_stroke(&mut self, frames: &[ReplayFrame], start: usize, end: usize) {
        self.stroke_start = Some(
            frames[start..end]
                .iter()
                .enumerate()
                .map(|(i, frame)| (start + i, frame.clone()))
                .collect(),
        );
    }

    /// Diffs the stroke snapshot against current state; a no-op if nothing
    /// changed. Absolute indices keep working even if the visible window
    /// slid during the hold.
    pub fn end_stroke(&mut self, frames: &[ReplayFrame]) {
        let Some(snapshot) = self.stroke_start.take() else {
            return;
        };

        let mut changes = Vec::new();

        for (idx, before) in snapshot {
            if frames[idx] != before {
                changes.push(FrameChange {
                    idx,
                    before,
                    after: frames[idx].clone(),
                });
            }
        }

        if !changes.is_empty() {
            self.push(changes);
        }
    }

    pub fn push(&mut self, changes: Vec<FrameChange>) {
        self.undo.push(changes);
        self.redo.clear();
    }

    /// Applies `before` states; returns the touched frame range.
    pub fn undo(&mut self, frames: &mut [ReplayFrame]) -> Option<(usize, usize)> {
        let changes = self.undo.pop()?;

        let (mut min, mut max) = (usize::MAX, 0);
        for change in &changes {
            frames[change.idx] = change.before.clone();
            min = min.min(change.idx);
            max = max.max(change.idx);
        }
        self.redo.push(changes);

        Some((min, max))
    }

    /// Applies `after` states; returns the touched frame range.
    pub fn redo(&mut self, frames: &mut [ReplayFrame]) -> Option<(usize, usize)> {
        let changes = self.redo.pop()?;

        let (mut min, mut max) = (usize::MAX, 0);
        for change in &changes {
            frames[change.idx] = change.after.clone();
            min = min.min(change.idx);
            max = max.max(change.idx);
        }
        self.undo.push(changes);

        Some((min, max))
    }
}
