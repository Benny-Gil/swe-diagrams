//! Text width estimate. The widths come from the advance widths of the
//! bundled Geist faces, in 1/1000 em, for U+0020..U+007E and U+00A0..U+017F.
//! A viewer without Geist uses a fallback face, so each width is at least
//! the Helvetica width, and the sum gets a small margin.

use crate::model::idiv;

/// Helvetica advance widths for U+0020..U+007E, the common fallback.
#[rustfmt::skip]
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556,
    278, 278, 584, 584, 584, 556, 1015,
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833,
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611,
    278, 278, 278, 469, 556, 333,
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833,
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500,
    334, 260, 334, 584,
];

/// Generated from rust/fonts/Geist-Regular.ttf and Geist-Bold.ttf. Index:
/// code point - 0x20 for ASCII, then code point - 0xa0 + 95. Zero means
/// that the face has no glyph.
#[rustfmt::skip]
const REGULAR: [u16; 319] = [
    250, 213, 346, 478, 629, 802, 620, 178, 274, 274, 430, 558, 201, 419, 201, 480,
    663, 384, 619, 613, 615, 626, 593, 524, 604, 593, 297, 297, 544, 540, 544, 559,
    906, 668, 680, 703, 694, 603, 590, 700, 713, 270, 597, 640, 580, 877, 743, 739,
    650, 733, 672, 640, 552, 689, 667, 945, 606, 576, 544, 347, 455, 347, 426, 557,
    248, 551, 595, 546, 595, 561, 395, 594, 581, 244, 260, 590, 267, 877, 581, 573,
    595, 595, 379, 520, 392, 575, 536, 819, 585, 537, 537, 389, 264, 389, 523, 250,
    213, 546, 611, 624, 576, 264, 549, 350, 828, 401, 533, 500, 0, 635, 356, 404,
    538, 348, 332, 248, 575, 596, 201, 317, 242, 419, 537, 732, 800, 782, 559, 668,
    668, 668, 668, 668, 669, 972, 703, 603, 603, 603, 603, 270, 270, 270, 270, 694,
    743, 739, 739, 739, 739, 739, 502, 739, 689, 689, 689, 689, 576, 620, 587, 551,
    551, 551, 551, 551, 551, 919, 546, 561, 561, 561, 561, 244, 244, 244, 244, 573,
    581, 573, 573, 573, 573, 573, 540, 573, 575, 575, 575, 575, 537, 595, 537, 668,
    551, 668, 551, 668, 551, 703, 546, 703, 546, 703, 546, 703, 546, 694, 662, 694,
    595, 603, 561, 0, 0, 603, 561, 603, 561, 603, 561, 700, 594, 700, 594, 700,
    594, 700, 594, 713, 581, 713, 581, 270, 244, 270, 244, 0, 0, 270, 244, 270,
    244, 847, 494, 597, 260, 640, 590, 0, 580, 267, 580, 267, 580, 318, 0, 0,
    580, 267, 743, 581, 743, 581, 743, 581, 0, 743, 581, 739, 573, 0, 0, 739,
    573, 1088, 958, 672, 379, 672, 379, 672, 379, 640, 520, 640, 520, 640, 520, 640,
    520, 552, 392, 552, 399, 552, 392, 689, 575, 689, 575, 689, 575, 689, 575, 689,
    575, 689, 575, 945, 819, 576, 537, 576, 544, 537, 544, 537, 544, 537, 0,
];
#[rustfmt::skip]
const BOLD: [u16; 319] = [
    228, 257, 390, 589, 670, 825, 706, 203, 323, 323, 422, 570, 236, 417, 236, 522,
    693, 449, 653, 650, 656, 671, 627, 544, 664, 631, 311, 311, 550, 552, 550, 591,
    962, 730, 703, 734, 716, 622, 604, 738, 721, 300, 627, 689, 589, 915, 750, 776,
    672, 769, 697, 681, 599, 703, 730, 1015, 688, 631, 594, 390, 501, 390, 461, 561,
    278, 594, 634, 598, 634, 605, 447, 634, 611, 281, 331, 647, 313, 900, 611, 618,
    634, 634, 425, 570, 445, 607, 609, 849, 650, 586, 583, 408, 294, 408, 523, 228,
    257, 598, 647, 633, 631, 294, 606, 432, 822, 432, 645, 512, 0, 649, 373, 428,
    550, 372, 369, 278, 607, 661, 236, 328, 282, 446, 655, 790, 846, 849, 591, 730,
    730, 730, 730, 730, 736, 1042, 734, 622, 622, 622, 622, 300, 300, 300, 300, 716,
    750, 776, 776, 776, 776, 776, 540, 776, 703, 703, 703, 703, 631, 642, 656, 594,
    594, 594, 594, 594, 594, 959, 598, 605, 605, 605, 605, 281, 281, 281, 281, 618,
    611, 618, 618, 618, 618, 618, 564, 618, 607, 607, 607, 607, 586, 624, 586, 730,
    594, 730, 594, 730, 594, 734, 598, 734, 598, 734, 598, 734, 598, 716, 741, 716,
    634, 622, 605, 0, 0, 622, 605, 622, 605, 622, 605, 738, 634, 738, 634, 738,
    634, 738, 634, 721, 611, 721, 611, 300, 281, 300, 281, 0, 0, 300, 281, 300,
    281, 913, 585, 627, 331, 689, 647, 0, 589, 313, 589, 313, 589, 390, 0, 0,
    589, 313, 750, 611, 750, 611, 750, 611, 0, 750, 611, 776, 618, 0, 0, 776,
    618, 1120, 998, 697, 425, 697, 425, 697, 425, 681, 570, 681, 570, 681, 570, 681,
    570, 599, 445, 599, 450, 599, 445, 703, 608, 703, 607, 703, 607, 703, 607, 703,
    607, 703, 607, 1015, 849, 631, 586, 631, 594, 583, 594, 583, 594, 583, 0,
];

/// Advance width of one character, in 1/1000 em.
fn advance(cp: u32, bold: bool) -> i64 {
    let table = if bold { &BOLD } else { &REGULAR };
    let geist = match cp {
        32..=126 => table[(cp - 32) as usize],
        0xa0..=0x17f => table[(cp - 0xa0 + 95) as usize],
        _ => 0,
    } as i64;
    let other = match cp {
        // A bold fallback face is about 6% wider than the regular table.
        32..=126 => HELVETICA[(cp - 32) as usize] as i64 * if bold { 106 } else { 100 } / 100,
        // Geist has most Latin letters. Their fallback widths are close.
        _ if geist > 0 => 0,
        0..=0x2e7f => 600,
        _ => 1000,
    };
    geist.max(other)
}

/// Width in pixels, rounded up. The 3% margin covers faces that are wider
/// than the two tables.
pub fn measure(s: &str, size: i64, bold: bool) -> i64 {
    let mut sum: i64 = 0;
    for ch in s.chars() {
        let cp = ch as u32;
        if cp < 32 {
            continue;
        }
        sum += advance(cp, bold);
    }
    idiv(sum * size * 103 + 99999, 100000)
}

/// Width in pixels of text in the mono face, rounded up. `size10` is the
/// font size times 10. The advance is 0.6 em, and the labels have a letter
/// spacing of 0.04 em.
pub fn measure_mono(s: &str, size10: i64) -> i64 {
    let mut sum = 0;
    for ch in s.chars() {
        let cp = ch as u32;
        if cp < 32 {
            continue;
        }
        sum += if cp >= 0x2e80 { 1040 } else { 640 };
    }
    idiv(sum * size10 + 9999, 10000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_rounds_up() {
        assert_eq!(measure("", 13, true), 0);
        // Geist Bold A is 730: 730 * 13 * 103 = 977470, and that is 9.77 pixels.
        assert_eq!(measure("A", 13, true), 10);
        // Geist A is 668: 668 * 11 * 103 = 756844, and that is 7.57 pixels.
        assert_eq!(measure("A", 11, false), 8);
    }

    #[test]
    fn measure_has_three_width_classes() {
        // A control character has no width.
        assert_eq!(measure("\t", 10, false), 0);
        // Geist e with an acute accent is 561: 561 * 10 * 103 = 577830.
        assert_eq!(measure("\u{e9}", 10, false), 6);
        // 1000 * 10 * 103 = 1030000.
        assert_eq!(measure("\u{65e5}", 10, false), 11);
    }

    #[test]
    fn each_width_covers_geist_and_helvetica() {
        // Geist digits are wider than Helvetica digits.
        assert_eq!(advance('0' as u32, false), 663);
        // Helvetica i is wider than Geist i.
        assert_eq!(advance('i' as u32, false), 244);
        assert_eq!(advance('f' as u32, false), 395);
        assert_eq!(advance('l' as u32, false), 267);
        assert_eq!(advance('I' as u32, false), 278);
    }
}
