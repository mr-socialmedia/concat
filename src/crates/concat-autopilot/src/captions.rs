//! Captions for speech, laid out for a phone held upright.
//!
//! A transcriber returns one line of text per stretch of speech, however
//! long. On a 9:16 frame that line must be broken into a page or two that
//! can be read at a glance, each shown for as long as it takes to read.
//!
//! Width is counted in letters as a reader sees them: Arabic's vowel marks,
//! shadda and tanwin sit on a letter and add no width, so counting them
//! would break lines far too early.

/// One stretch of speech with its words, in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    /// When it begins.
    pub start: f64,
    /// When it ends.
    pub end: f64,
    /// What was said.
    pub text: String,
}

/// How captions are laid out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptionOptions {
    /// The widest a line may be, in letters. About 22 reads comfortably at
    /// the size captions use on a 1080-wide vertical frame.
    pub max_chars: usize,
    /// How many lines a caption may have; two is what a reader takes in at
    /// a glance.
    pub max_lines: usize,
    /// The shortest a caption stays up, so a short word is not a flicker.
    pub min_duration: f64,
    /// The fastest it may be read, in letters per second. Arabic is read
    /// more slowly than the speech that carries it, so a caption that would
    /// go faster is held longer, as far as the next caption allows.
    pub max_cps: f64,
}

impl Default for CaptionOptions {
    fn default() -> Self {
        CaptionOptions {
            max_chars: 22,
            max_lines: 2,
            min_duration: 0.8,
            max_cps: 15.0,
        }
    }
}

/// One caption as it is shown: its lines joined by a line break.
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    /// When it appears.
    pub start: f64,
    /// When it goes.
    pub end: f64,
    /// The words, one line to a row.
    pub text: String,
}

/// Whether `c` takes no room of its own in a line: Arabic vowel marks and
/// Quranic annotation signs, combining marks, the tatweel stretch, and the
/// invisible controls that steer the direction of text.
fn takes_no_width(c: char) -> bool {
    matches!(c,
        '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}' | '\u{061C}'
        | '\u{06D6}'..='\u{06DC}' | '\u{06DF}'..='\u{06E4}' | '\u{06E7}' | '\u{06E8}'
        | '\u{06EA}'..='\u{06ED}' | '\u{0300}'..='\u{036F}'
        | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// How wide `text` is, in letters as a reader counts them.
pub fn display_len(text: &str) -> usize {
    text.chars().filter(|c| !takes_no_width(*c)).count()
}

/// Packs `words` into pages of at most `lines` lines of at most `width`
/// letters. A word wider than a line gets a line to itself rather than
/// being split, since splitting an Arabic word breaks its joins.
fn paginate(words: &[&str], width: usize, lines: usize) -> Vec<Vec<String>> {
    let mut pages: Vec<Vec<String>> = Vec::new();
    let mut page: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut line_len = 0;
    for word in words {
        let w = display_len(word);
        if !line.is_empty() && line_len + 1 + w > width {
            page.push(std::mem::take(&mut line));
            line_len = 0;
            if page.len() == lines {
                pages.push(std::mem::take(&mut page));
            }
        }
        if !line.is_empty() {
            line.push(' ');
            line_len += 1;
        }
        line.push_str(word);
        line_len += w;
    }
    if !line.is_empty() {
        page.push(line);
    }
    if !page.is_empty() {
        pages.push(page);
    }
    pages
}

/// Lays out `segments` as captions: broken into pages, each page given the
/// share of its segment's time that its letters make up, then held long
/// enough to read without running into the next one.
///
/// Segments that are empty, not finite or end before they start are
/// skipped. The result is in order and no caption overlaps the next.
pub fn build(segments: &[Segment], options: &CaptionOptions) -> Vec<Caption> {
    let mut ordered: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.start.is_finite() && s.end.is_finite() && s.end > s.start)
        .collect();
    ordered.sort_by(|a, b| a.start.total_cmp(&b.start));

    let mut captions: Vec<Caption> = Vec::new();
    for segment in ordered {
        let words: Vec<&str> = segment.text.split_whitespace().collect();
        let pages = paginate(&words, options.max_chars.max(1), options.max_lines.max(1));
        let weights: Vec<usize> = pages
            .iter()
            .map(|page| page.iter().map(|l| display_len(l)).sum::<usize>().max(1))
            .collect();
        let total: usize = weights.iter().sum();
        let mut at = segment.start;
        for (page, weight) in pages.iter().zip(&weights) {
            let share = (segment.end - segment.start) * (*weight as f64) / (total as f64);
            captions.push(Caption {
                start: at,
                end: at + share,
                text: page.join("\n"),
            });
            at += share;
        }
    }
    hold(&mut captions, options);
    captions
}

/// Holds each caption long enough to read, but never into the next one.
fn hold(captions: &mut [Caption], options: &CaptionOptions) {
    for i in 0..captions.len() {
        let letters = captions[i].text.split('\n').map(display_len).sum::<usize>() as f64;
        let wanted = options.min_duration.max(letters / options.max_cps.max(0.1));
        let mut end = captions[i].end.max(captions[i].start + wanted);
        if let Some(next) = captions.get(i + 1) {
            end = end.min(next.start);
        }
        // Never shorten what the speech gave it.
        captions[i].end = end.max(
            captions[i]
                .end
                .min(captions.get(i + 1).map_or(f64::MAX, |n| n.start)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: f64, end: f64, text: &str) -> Segment {
        Segment {
            start,
            end,
            text: text.to_owned(),
        }
    }

    /// Vowel marks, shadda and tanwin add no width, so they are not counted.
    #[test]
    fn marks_do_not_widen_a_line() {
        assert_eq!(display_len("حرّر"), 3); // ح ر ر: the shadda is a mark on the second ر
        assert_eq!(display_len("مرحبًا"), 5);
        assert_eq!(display_len("اللَّه"), 4);
        assert_eq!(display_len("كـــتاب"), 4); // tatweel stretches, it does not add a letter
        assert_eq!(display_len("CutZone"), 7);
        assert_eq!(display_len("\u{200F}مرحبا\u{200E}"), 5);
    }

    /// A short line is one caption, as said.
    #[test]
    fn a_short_line_is_one_caption() {
        let c = build(
            &[seg(1.0, 3.0, "مرحبًا بكم في CutZone")],
            &CaptionOptions::default(),
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, "مرحبًا بكم في CutZone"); // 20 letters: one line
        assert_eq!((c[0].start, c[0].end), (1.0, 3.0));
    }

    /// A long line breaks into pages whose lines fit, keeping every word in
    /// order and the whole of the segment's time.
    #[test]
    fn a_long_line_becomes_pages_that_fit_in_order() {
        let text = "هذا نص عربي طويل يختبر التفاف الأسطر داخل الإطار العمودي بدون تقطيع الحروف المتصلة أو عكس ترتيب الكلمات";
        let o = CaptionOptions {
            min_duration: 0.0,
            max_cps: 1000.0,
            ..CaptionOptions::default()
        };
        let c = build(&[seg(0.0, 12.0, text)], &o);
        assert!(c.len() >= 2);
        let said: Vec<&str> = text.split_whitespace().collect();
        let shown: Vec<&str> = c.iter().flat_map(|c| c.text.split_whitespace()).collect();
        assert_eq!(said, shown, "every word, in order");
        for caption in &c {
            let lines: Vec<&str> = caption.text.split('\n').collect();
            assert!(lines.len() <= 2);
            assert!(lines.iter().all(|l| display_len(l) <= 22), "{lines:?}");
        }
        assert_eq!(c.first().unwrap().start, 0.0);
        assert!((c.last().unwrap().end - 12.0).abs() < 1e-9);
        for pair in c.windows(2) {
            assert!(
                (pair[0].end - pair[1].start).abs() < 1e-9,
                "pages are contiguous"
            );
        }
    }

    /// A word wider than a line is kept whole, alone on its line.
    #[test]
    fn a_word_wider_than_a_line_is_not_split() {
        let c = build(
            &[seg(0.0, 4.0, "قصير استثنائيةجداجداجداجداجداجداجدا قصير")],
            &CaptionOptions::default(),
        );
        let words: Vec<&str> = c.iter().flat_map(|c| c.text.split_whitespace()).collect();
        assert_eq!(
            words,
            vec!["قصير", "استثنائيةجداجداجداجداجداجداجدا", "قصير"]
        );
    }

    /// A caption too quick to read is held, but never into the next one.
    #[test]
    fn a_quick_caption_is_held_but_not_into_the_next() {
        let o = CaptionOptions::default();
        let c = build(&[seg(0.0, 0.2, "نعم"), seg(0.5, 3.0, "تمام جدا")], &o);
        assert_eq!(c.len(), 2);
        assert!(
            (c[0].end - 0.5).abs() < 1e-9,
            "held up to the next caption, not past it: {}",
            c[0].end
        );
        assert!(c[0].end <= c[1].start + 1e-9);
        // With room, it is held to the minimum.
        let alone = build(&[seg(0.0, 0.2, "نعم")], &o);
        assert!((alone[0].end - 0.8).abs() < 1e-9);
    }

    /// Nothing to say, or nonsense timing, makes no caption.
    #[test]
    fn empty_and_nonsense_segments_make_nothing() {
        let c = build(
            &[
                seg(0.0, 1.0, "   "),
                seg(f64::NAN, 2.0, "x"),
                seg(3.0, 2.0, "y"),
                seg(4.0, 5.0, "حسنا"),
            ],
            &CaptionOptions::default(),
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, "حسنا");
    }

    /// Segments given out of order come out in order.
    #[test]
    fn segments_are_ordered_by_their_start() {
        let c = build(
            &[seg(5.0, 6.0, "ثانيا"), seg(1.0, 2.0, "أولا")],
            &CaptionOptions::default(),
        );
        assert_eq!(c[0].text, "أولا");
        assert_eq!(c[1].text, "ثانيا");
    }

    /// For random talk, in any mix of short and long words: no overlap, every
    /// caption fits and every word survives.
    #[test]
    fn captions_hold_for_random_talk() {
        let vocabulary = [
            "نعم",
            "مرحبًا",
            "حرّر",
            "CutZone",
            "استثنائي",
            "في",
            "الفيديو",
            "أبدع",
            "تسجيل",
            "رائع",
            "12",
            "٢٠٢٦",
        ];
        let mut seed = 0xabad_cafe_u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };
        let o = CaptionOptions::default();
        for case in 0..300 {
            let mut segments = Vec::new();
            let mut at = 0.0;
            for _ in 0..(1 + next() % 6) {
                let n = 1 + next() % 30;
                let text: Vec<&str> = (0..n)
                    .map(|_| vocabulary[next() % vocabulary.len()])
                    .collect();
                let len = 0.3 + (next() % 800) as f64 / 100.0;
                segments.push(seg(at, at + len, &text.join(" ")));
                at += len + (next() % 300) as f64 / 100.0;
            }
            let said: Vec<String> = segments
                .iter()
                .flat_map(|s| s.text.split_whitespace().map(str::to_owned))
                .collect();
            let captions = build(&segments, &o);
            let shown: Vec<String> = captions
                .iter()
                .flat_map(|c| c.text.split_whitespace().map(str::to_owned))
                .collect();
            assert_eq!(said, shown, "case {case}");
            for pair in captions.windows(2) {
                assert!(pair[0].end <= pair[1].start + 1e-9, "case {case}: overlap");
            }
            for caption in &captions {
                assert!(caption.end > caption.start, "case {case}");
                let lines: Vec<&str> = caption.text.split('\n').collect();
                assert!(lines.len() <= o.max_lines, "case {case}");
                assert!(
                    lines
                        .iter()
                        .all(|l| display_len(l) <= o.max_chars || !l.contains(' ')),
                    "case {case}: {lines:?}"
                );
            }
        }
    }
}
