// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Package version comparison with the same rules as pacman's `vercmp`.
//!
//! Kept in pure Rust so the domain logic can be tested without libalpm.

use std::cmp::Ordering;

/// Compares two full package versions (`[epoch:]version[-release]`).
///
/// The release is only compared when both sides have one, as in pacman.
pub fn vercmp(a: &str, b: &str) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let (ea, va, ra) = split_evr(a);
    let (eb, vb, rb) = split_evr(b);
    rpmvercmp(ea, eb)
        .then_with(|| rpmvercmp(va, vb))
        .then_with(|| match (ra, rb) {
            (Some(ra), Some(rb)) => rpmvercmp(ra, rb),
            _ => Ordering::Equal,
        })
}

fn split_evr(evr: &str) -> (&str, &str, Option<&str>) {
    let digits = evr.bytes().take_while(u8::is_ascii_digit).count();
    let (epoch, rest) = if evr.as_bytes().get(digits) == Some(&b':') {
        let epoch = &evr[..digits];
        (if epoch.is_empty() { "0" } else { epoch }, &evr[digits + 1..])
    } else {
        ("0", evr)
    };
    match rest.rfind('-') {
        Some(i) => (epoch, &rest[..i], Some(&rest[i + 1..])),
        None => (epoch, rest, None),
    }
}

fn rpmvercmp(a: &str, b: &str) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let a = a.as_bytes();
    let b = b.as_bytes();
    let (mut one, mut two) = (0usize, 0usize);

    while one < a.len() && two < b.len() {
        let (start1, start2) = (one, two);
        while one < a.len() && !a[one].is_ascii_alphanumeric() {
            one += 1;
        }
        while two < b.len() && !b[two].is_ascii_alphanumeric() {
            two += 1;
        }
        if one >= a.len() || two >= b.len() {
            break;
        }
        // Different separator lengths decide the comparison.
        let (sep1, sep2) = (one - start1, two - start2);
        if sep1 != sep2 {
            return sep1.cmp(&sep2);
        }

        let isnum = a[one].is_ascii_digit();
        let class = |c: &u8| {
            if isnum {
                c.is_ascii_digit()
            } else {
                c.is_ascii_alphabetic()
            }
        };
        let end1 = one + a[one..].iter().take_while(|c| class(c)).count();
        let end2 = two + b[two..].iter().take_while(|c| class(c)).count();

        if two == end2 {
            // Numeric segments are newer than alphabetic ones.
            return if isnum { Ordering::Greater } else { Ordering::Less };
        }

        let mut left = &a[one..end1];
        let mut right = &b[two..end2];
        if isnum {
            left = trim_zeros(left);
            right = trim_zeros(right);
            match left.len().cmp(&right.len()) {
                Ordering::Equal => {}
                other => return other,
            }
        }
        match left.cmp(right) {
            Ordering::Equal => {}
            other => return other,
        }
        one = end1;
        two = end2;
    }

    let rest1 = &a[one.min(a.len())..];
    let rest2 = &b[two.min(b.len())..];
    if rest1.is_empty() && rest2.is_empty() {
        return Ordering::Equal;
    }
    // A remaining alphabetic part never beats an empty string.
    let one_alpha = rest1.first().is_some_and(u8::is_ascii_alphabetic);
    let two_alpha = rest2.first().is_some_and(u8::is_ascii_alphabetic);
    if (rest1.is_empty() && !two_alpha) || one_alpha {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

fn trim_zeros(s: &[u8]) -> &[u8] {
    let n = s.iter().take_while(|&&c| c == b'0').count();
    &s[n..]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values produced by pacman's `vercmp` 7.1.
    const CASES: &[(&str, &str, i8)] = &[
        ("1.0", "1.0", 0),
        ("1.0", "1.1", -1),
        ("1.1", "1.0", 1),
        ("1.0-1", "1.0-2", -1),
        ("1:1.0", "2.0", 1),
        ("1.0", "1:0.5", -1),
        ("1.0a", "1.0", -1),
        ("1.0", "1.0a", 1),
        ("1.0alpha", "1.0beta", -1),
        ("1.0.1", "1.0", 1),
        ("1.0", "1.0.1", -1),
        ("1.0..1", "1.0.1", 1),
        ("1.0.0", "1.0", 1),
        ("2.0", "10.0", -1),
        ("010", "10", 0),
        ("1.0-1", "1.0", 0),
        ("1.0", "1.0-1", 0),
        ("1.0rc1", "1.0", -1),
        ("1.0", "1.0rc1", 1),
        ("1.0_1", "1.0.1", 0),
        ("1.0+1", "1.0.1", 0),
        ("a", "b", -1),
        ("1.a", "1.1", -1),
        ("1:1.0-1", "1:1.0-1", 0),
        ("0:1.0", "1.0", 0),
        ("4.1.4-1", "4.1.1-1", 1),
        ("26.08.2", "26.08.1", 1),
        ("1.0a1", "1.0a", 1),
        ("1.0.a", "1.0.1", -1),
        ("6.6.2-1", "6.6.10-1", -1),
        ("r1234.abc", "r1235.abc", -1),
        ("1.0~rc1", "1.0", 1),
        ("1.0-", "1.0", 0),
        ("1:2.0-1", "1:2.0", 0),
        ("1.0-1.1", "1.0-1", 1),
    ];

    #[test]
    fn matches_pacman_vercmp() {
        for &(a, b, want) in CASES {
            let want = want.cmp(&0);
            assert_eq!(vercmp(a, b), want, "vercmp({a}, {b})");
            assert_eq!(vercmp(b, a), want.reverse(), "vercmp({b}, {a})");
        }
    }

    #[test]
    fn empty_epoch_counts_as_zero() {
        assert_eq!(vercmp(":1.0", "1.0"), Ordering::Equal);
    }
}
