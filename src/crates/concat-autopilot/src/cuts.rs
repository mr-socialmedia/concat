//! Cutting the silence out of talk.
//!
//! A voice detector says where someone speaks; this turns that into the
//! stretches of the footage to keep, and says where any moment of the
//! original lands once the rest is gone - which is what keeps captions on
//! the words they belong to after the cut.
//!
//! Everything here is plain arithmetic on seconds, with no audio, no
//! project and no model, so the rules are tested to the digit.

/// Two times closer than this are the same time. A frame at 120 fps is
/// 8.3 ms, so a microsecond is far below anything a person could see.
const EPS: f64 = 1e-6;

/// A stretch of time, in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    /// Where it begins.
    pub start: f64,
    /// Where it ends, after `start`.
    pub end: f64,
}

impl Span {
    /// A span from `start` to `end`.
    pub fn new(start: f64, end: f64) -> Span {
        Span { start, end }
    }

    /// How long it lasts.
    pub fn len(&self) -> f64 {
        self.end - self.start
    }
}

/// What counts as speech worth keeping, and how much air is left around it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutOptions {
    /// Sound kept before speech begins. Voice detectors are tight on the
    /// attack of a word, and a clipped first syllable is the first thing a
    /// viewer notices.
    pub lead: f64,
    /// Sound kept after speech ends: the breath and the fall of the voice.
    pub tail: f64,
    /// A pause shorter than this stays, because removing it makes speech
    /// sound rushed and the edit audible. Longer pauses are cut.
    pub min_gap: f64,
    /// A detected burst shorter than this is a click or a cough, not speech.
    pub min_speech: f64,
}

impl Default for CutOptions {
    fn default() -> Self {
        CutOptions {
            lead: 0.10,
            tail: 0.20,
            min_gap: 0.45,
            min_speech: 0.20,
        }
    }
}

/// Why no cut could be planned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The footage has no usable length.
    BadDuration,
    /// No speech was heard. Cutting everything would hand back an empty
    /// video, so the caller is told instead.
    NoSpeech,
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::BadDuration => f.write_str("the footage has no length"),
            PlanError::NoSpeech => f.write_str("no speech was heard, so nothing is cut"),
        }
    }
}

impl std::error::Error for PlanError {}

/// The stretches of the original to keep, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// How long the original is.
    pub total: f64,
    /// What stays: sorted, apart from one another, inside `0..total`.
    pub keep: Vec<Span>,
}

/// Plans the cut for footage `total` seconds long whose speech is `speech`,
/// in any order and possibly overlapping. Anything not a finite span is
/// ignored, so a detector's odd output cannot break the plan.
pub fn plan(speech: &[Span], total: f64, options: CutOptions) -> Result<Plan, PlanError> {
    if !(total.is_finite() && total > EPS) {
        return Err(PlanError::BadDuration);
    }
    let mut spans: Vec<Span> = speech
        .iter()
        .filter(|s| s.start.is_finite() && s.end.is_finite() && s.end > s.start)
        .map(|s| Span::new(s.start.clamp(0.0, total), s.end.clamp(0.0, total)))
        .filter(|s| s.len() + EPS >= options.min_speech && s.len() > EPS)
        .collect();
    spans.sort_by(|a, b| a.start.total_cmp(&b.start));
    if spans.is_empty() {
        return Err(PlanError::NoSpeech);
    }

    let mut keep: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        let padded = Span::new(
            (span.start - options.lead).max(0.0),
            (span.end + options.tail).min(total),
        );
        match keep.last_mut() {
            // Close enough to the last stretch to be one breath with it.
            Some(last) if padded.start - last.end < options.min_gap => {
                last.end = last.end.max(padded.end);
            }
            _ => keep.push(padded),
        }
    }
    Ok(Plan { total, keep })
}

impl Plan {
    /// How much of the original stays.
    pub fn kept(&self) -> f64 {
        self.keep.iter().map(Span::len).sum()
    }

    /// How much is cut.
    pub fn removed(&self) -> f64 {
        self.total - self.kept()
    }

    /// The stretches that go, in order: what lies between the kept ones, and
    /// before the first and after the last. Slivers below a microsecond are
    /// not cuts.
    pub fn cuts(&self) -> Vec<Span> {
        let mut cuts = Vec::new();
        let mut at = 0.0;
        for kept in &self.keep {
            if kept.start - at > EPS {
                cuts.push(Span::new(at, kept.start));
            }
            at = kept.end;
        }
        if self.total - at > EPS {
            cuts.push(Span::new(at, self.total));
        }
        cuts
    }

    /// Where the moment `t` of the original is in the cut footage. A moment
    /// that was itself cut lands where the next kept stretch begins, so the
    /// mapping never goes backwards.
    pub fn remap_point(&self, t: f64) -> f64 {
        let mut before = 0.0;
        for kept in &self.keep {
            if t < kept.start {
                return before;
            }
            if t <= kept.end {
                return before + (t - kept.start);
            }
            before += kept.len();
        }
        before
    }

    /// Where a stretch of the original is in the cut footage: from where its
    /// first kept moment lands to where its last does. `None` when none of
    /// it was kept. A stretch with a pause cut out of its middle comes back
    /// as one stretch, the pause gone, as it plays.
    pub fn remap_range(&self, start: f64, end: f64) -> Option<Span> {
        let mut first: Option<f64> = None;
        let mut last = 0.0;
        for kept in &self.keep {
            let from = start.max(kept.start);
            let to = end.min(kept.end);
            if to - from > EPS {
                first.get_or_insert(from);
                last = to;
            }
        }
        let first = first?;
        let span = Span::new(self.remap_point(first), self.remap_point(last));
        (span.len() > EPS).then_some(span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(a: f64, b: f64) -> Span {
        Span::new(a, b)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// Speech gets air on both sides, and two stretches far apart stay two.
    #[test]
    fn speech_is_padded_and_far_apart_stretches_stay_apart() {
        let plan = plan(&[s(2.0, 4.0), s(10.0, 12.0)], 20.0, CutOptions::default()).unwrap();
        assert_eq!(plan.keep, vec![s(1.9, 4.2), s(9.9, 12.2)]);
        assert!(close(plan.kept(), 2.3 + 2.3));
        assert!(close(plan.removed(), 20.0 - 4.6));
    }

    /// A short pause is part of speaking and stays; a long one is cut.
    #[test]
    fn a_short_pause_stays_and_a_long_one_goes() {
        // Padded, the gap between 4.0 and 4.3 is 0.0: one breath.
        let one = plan(&[s(1.0, 4.0), s(4.3, 6.0)], 10.0, CutOptions::default()).unwrap();
        assert_eq!(one.keep.len(), 1);
        assert!(close(one.keep[0].start, 0.9) && close(one.keep[0].end, 6.2));
        // 0.6 s of silence between speech is padded down to 0.3 s: under
        // min_gap (0.45), so it stays.
        let kept = plan(&[s(1.0, 3.0), s(3.6, 5.0)], 10.0, CutOptions::default()).unwrap();
        assert_eq!(kept.keep.len(), 1);
        // 2 s of silence is cut.
        let cut = plan(&[s(1.0, 3.0), s(5.0, 6.0)], 10.0, CutOptions::default()).unwrap();
        assert_eq!(cut.keep.len(), 2);
    }

    /// Edges of the footage clamp rather than going outside it.
    #[test]
    fn padding_never_leaves_the_footage() {
        let plan = plan(&[s(0.0, 0.5), s(9.8, 10.0)], 10.0, CutOptions::default()).unwrap();
        assert!(plan.keep.first().unwrap().start >= 0.0);
        assert!(plan.keep.last().unwrap().end <= 10.0);
        // Speech reported past the end is clipped to it.
        let over = super::plan(&[s(9.0, 14.0)], 10.0, CutOptions::default()).unwrap();
        assert!(close(over.keep[0].end, 10.0));
    }

    /// A click is not speech; a detector's nonsense is ignored; silence is
    /// refused rather than turned into an empty video.
    #[test]
    fn blips_and_nonsense_are_ignored_and_silence_is_refused() {
        let o = CutOptions::default();
        assert_eq!(plan(&[s(1.0, 1.1)], 10.0, o), Err(PlanError::NoSpeech));
        assert_eq!(plan(&[], 10.0, o), Err(PlanError::NoSpeech));
        let mixed = [
            s(f64::NAN, 2.0),
            s(3.0, f64::INFINITY),
            s(5.0, 4.0),
            s(2.0, 4.0),
        ];
        let ok = plan(&mixed, 10.0, o).unwrap();
        assert_eq!(ok.keep, vec![s(1.9, 4.2)]);
        assert_eq!(plan(&[s(1.0, 2.0)], 0.0, o), Err(PlanError::BadDuration));
        assert_eq!(
            plan(&[s(1.0, 2.0)], f64::NAN, o),
            Err(PlanError::BadDuration)
        );
    }

    /// The cuts are exactly what the kept stretches leave out.
    #[test]
    fn cuts_are_the_complement_of_the_kept_stretches() {
        let plan = plan(&[s(2.0, 4.0), s(10.0, 12.0)], 20.0, CutOptions::default()).unwrap();
        let cuts = plan.cuts();
        assert_eq!(cuts, vec![s(0.0, 1.9), s(4.2, 9.9), s(12.2, 20.0)]);
        assert!(close(
            cuts.iter().map(Span::len).sum::<f64>(),
            plan.removed()
        ));
        // Keeping everything leaves nothing to cut.
        let all = Plan {
            total: 5.0,
            keep: vec![s(0.0, 5.0)],
        };
        assert!(all.cuts().is_empty());
    }

    /// A caption's time is where its words now are, which is what stops it
    /// drifting off the voice after the cut.
    #[test]
    fn a_moment_lands_where_it_plays_after_the_cut() {
        let plan = Plan {
            total: 20.0,
            keep: vec![s(1.0, 3.0), s(8.0, 11.0), s(15.0, 16.0)],
        };
        assert!(close(plan.remap_point(1.0), 0.0));
        assert!(close(plan.remap_point(2.5), 1.5));
        assert!(close(plan.remap_point(8.0), 2.0)); // first moment of the second stretch
        assert!(close(plan.remap_point(10.0), 4.0));
        assert!(close(plan.remap_point(15.5), 5.5));
        // A moment that was cut lands where the next kept stretch starts.
        assert!(close(plan.remap_point(5.0), 2.0));
        assert!(close(plan.remap_point(0.0), 0.0));
        assert!(close(plan.remap_point(20.0), plan.kept()));
    }

    /// A line spoken across a cut pause becomes one line with the pause
    /// gone, a line entirely in the cut has no place, and a line half in
    /// the cut keeps only the half that remains.
    #[test]
    fn a_range_follows_its_words_through_the_cut() {
        let plan = Plan {
            total: 20.0,
            keep: vec![s(1.0, 3.0), s(8.0, 11.0)],
        };
        let across = plan.remap_range(2.0, 9.0).unwrap();
        assert!(close(across.start, 1.0) && close(across.end, 3.0)); // 1 s + 1 s, the pause gone
        assert_eq!(plan.remap_range(4.0, 7.0), None);
        let half = plan.remap_range(0.0, 2.0).unwrap();
        assert!(close(half.start, 0.0) && close(half.end, 1.0));
        assert_eq!(plan.remap_range(12.0, 13.0), None);
    }

    /// A small deterministic generator, so the property tests need no crate
    /// and fail the same way every time.
    fn lcg(seed: &mut u64) -> f64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 33) as f64) / ((1u64 << 31) as f64)
    }

    /// For many random recordings: the kept stretches are ordered, apart by
    /// at least the minimum gap, inside the footage and cover every speech
    /// burst long enough to count; and the mapping never runs backwards.
    #[test]
    fn the_plan_holds_for_random_recordings() {
        let options = CutOptions::default();
        let mut seed = 0x5eed_cafe_u64;
        for case in 0..400 {
            let total = 5.0 + lcg(&mut seed) * 120.0;
            let mut speech = Vec::new();
            let mut at = lcg(&mut seed) * 3.0;
            while at < total {
                let len = 0.05 + lcg(&mut seed) * 4.0;
                speech.push(s(at, (at + len).min(total + 1.0)));
                at += len + lcg(&mut seed) * 3.0;
            }
            let Ok(plan) = plan(&speech, total, options) else {
                continue;
            };

            for pair in plan.keep.windows(2) {
                assert!(pair[0].end <= pair[1].start, "case {case}: overlap");
                assert!(
                    pair[1].start - pair[0].end >= options.min_gap - 1e-9,
                    "case {case}: gap"
                );
            }
            assert!(
                plan.keep
                    .iter()
                    .all(|k| k.start >= 0.0 && k.end <= total + 1e-9 && k.len() > 0.0)
            );
            assert!(close(plan.kept() + plan.removed(), total));
            for burst in speech
                .iter()
                .filter(|b| b.start < total && b.end.min(total) - b.start >= options.min_speech)
            {
                let end = burst.end.min(total);
                assert!(
                    plan.keep
                        .iter()
                        .any(|k| k.start <= burst.start + 1e-9 && k.end >= end - 1e-9),
                    "case {case}: speech {burst:?} was cut"
                );
            }
            let mut last = -1.0;
            for step in 0..=200 {
                let mapped = plan.remap_point(total * f64::from(step) / 200.0);
                assert!(
                    mapped >= last - 1e-9,
                    "case {case}: the mapping went backwards"
                );
                last = mapped;
            }
            assert!(close(plan.remap_point(total), plan.kept()));
        }
    }
}
