// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! How far a long step is, and roughly how long it still takes. Fed with
//! download bytes or counted items; times are seconds from any clock.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measure {
    Bytes { done: u64, total: u64 },
    Items { current: usize, total: usize },
}

#[derive(Debug, Clone, Default)]
pub struct Tracker {
    files: BTreeMap<String, (u64, u64)>,
    items: Option<(usize, usize)>,
    /// When the current kind of measure started.
    started: Option<f64>,
}

impl Tracker {
    /// A download made progress. Starts a new measurement after items.
    pub fn download(&mut self, file: &str, done: u64, total: u64, now: f64) {
        if self.items.is_some() || self.started.is_none() {
            self.items = None;
            self.files.clear();
            self.started = Some(now);
        }
        self.files.insert(file.to_owned(), (done, total.max(done)));
    }

    /// Item `current` of `total` (packages installed, apps built) started.
    pub fn item(&mut self, current: usize, total: usize, now: f64) {
        let same = self.items.is_some_and(|(_, t)| t == total);
        if !same {
            self.files.clear();
            self.started = Some(now);
        }
        self.items = Some((current, total));
    }

    pub fn measure(&self) -> Option<Measure> {
        if let Some((current, total)) = self.items {
            return (total > 0).then_some(Measure::Items { current, total });
        }
        let (done, total) = self
            .files
            .values()
            .fold((0, 0), |(d, t), (fd, ft)| (d + fd, t + ft));
        (total > 0).then_some(Measure::Bytes { done, total })
    }

    /// Between 0 and 1, or `None` when there is nothing to measure.
    #[allow(clippy::cast_precision_loss)] // progress needs no exact bytes
    pub fn fraction(&self) -> Option<f64> {
        match self.measure()? {
            Measure::Bytes { done, total } => Some(done as f64 / total as f64),
            Measure::Items { current, total } => Some(current.saturating_sub(1) as f64 / total as f64),
        }
    }

    /// Seconds left, once enough has happened to estimate: two seconds and
    /// some progress.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn remaining(&self, now: f64) -> Option<u64> {
        let elapsed = now - self.started?;
        let fraction = self.fraction()?;
        if elapsed < 2.0 || fraction <= 0.0 || fraction >= 1.0 {
            return None;
        }
        Some((elapsed / fraction * (1.0 - fraction)).round() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_add_up_and_estimate_the_rest() {
        let mut t = Tracker::default();
        assert_eq!(t.fraction(), None);
        t.download("core.db", 50, 100, 0.0);
        t.download("extra.db", 0, 300, 0.0);
        assert_eq!(t.measure(), Some(Measure::Bytes { done: 50, total: 400 }));
        assert_eq!(t.remaining(1.0), None);
        t.download("extra.db", 150, 300, 4.0);
        assert_eq!(t.fraction(), Some(0.5));
        assert_eq!(t.remaining(4.0), Some(4));
    }

    #[test]
    fn items_follow_downloads() {
        let mut t = Tracker::default();
        t.download("a.pkg", 10, 10, 0.0);
        t.item(1, 4, 10.0);
        assert_eq!(t.measure(), Some(Measure::Items { current: 1, total: 4 }));
        assert_eq!(t.fraction(), Some(0.0));
        t.item(3, 4, 14.0);
        assert_eq!(t.fraction(), Some(0.5));
        assert_eq!(t.remaining(14.0), Some(4));
        // A new download after items starts a new measurement.
        t.download("b.pkg", 1, 2, 20.0);
        assert_eq!(t.measure(), Some(Measure::Bytes { done: 1, total: 2 }));
        t.item(1, 0, 21.0);
        assert_eq!(t.fraction(), None);
    }
}
