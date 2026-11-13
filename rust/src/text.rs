//! Text width estimate. The harness has no font engine, so it uses the
//! advance widths of a common sans face, in 1/1000 em, for ASCII 32..126.

use crate::model::idiv;

#[rustfmt::skip]
const ADV: [u16; 95] = [
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

/// Width in pixels, rounded up. The 2% and 8% margins cover faces that are
/// wider than the table.
pub fn measure(s: &str, size: i64, bold: bool) -> i64 {
    let mut sum: i64 = 0;
    for ch in s.chars() {
        let cp = ch as u32;
        if cp < 32 {
            continue;
        }
        sum += if cp <= 126 {
            ADV[(cp - 32) as usize] as i64
        } else if cp >= 0x2e80 {
            1000
        } else {
            600
        };
    }
    idiv(sum * size * if bold { 108 } else { 102 } + 99999, 100000)
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
        // 667 * 13 * 108 = 936468, and that is 9.36 pixels.
        assert_eq!(measure("A", 13, true), 10);
        // 667 * 11 * 102 = 748374, and that is 7.48 pixels.
        assert_eq!(measure("A", 11, false), 8);
    }

    #[test]
    fn measure_has_three_width_classes() {
        // A control character has no width.
        assert_eq!(measure("\t", 10, false), 0);
        // 600 * 10 * 102 = 612000.
        assert_eq!(measure("é", 10, false), 7);
        // 1000 * 10 * 102 = 1020000.
        assert_eq!(measure("日", 10, false), 11);
    }
}
