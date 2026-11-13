//! The clamp's arithmetic (R-37), row by row.

use super::clamped;

const T: bool = true;
const F: bool = false;

#[test]
fn the_clamp_fits_the_docks_to_the_area() {
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    // (case, width, height, sizes, open, expected): `None` is left alone.
    type Row = (
        &'static str,
        f32,
        f32,
        [f32; 3],
        [bool; 3],
        [Option<f32>; 3],
    );
    #[rustfmt::skip]
    let rows: [Row; 14] = [
        // Both side docks are scaled in proportion: 900 : 900 in 1,000 px
        // leaves 680 for them, 340 each.
        ("both sides too wide", 1000., 800., [900., 900., 150.], [T, T, T], [Some(340.), Some(340.), None]),
        ("in proportion", 1000., 800., [600., 300., 150.], [T, T, T], [Some(453.333), Some(226.667), None]),
        // One open side dock alone gets the whole of the 480 px.
        ("one side dock open", 800., 800., [900., 440., 150.], [T, F, T], [Some(480.), None, None]),
        // Closed docks are not counted and not changed.
        ("every dock closed", 100., 100., [900., 900., 700.], [F, F, F], [None, None, None]),
        // Under 520 px wide the floor of 100 wins and the item area is
        // under 320 px.
        ("a window under 520 px", 400., 800., [300., 300., 150.], [T, T, T], [Some(100.), Some(100.), None]),
        ("a window under 320 px", 200., 800., [300., 300., 150.], [T, T, T], [Some(100.), Some(100.), None]),
        // A bottom dock leaves 200 px, or its floor.
        ("a bottom dock taller than the window", 1200., 500., [290., 440., 700.], [T, T, T], [None, None, Some(300.)]),
        ("a window shorter than 300 px", 1200., 250., [290., 440., 150.], [T, T, T], [None, None, Some(100.)]),
        // A dock exactly at the limit is already right.
        ("sides exactly at the limit", 1000., 800., [340., 340., 150.], [T, T, T], [None, None, None]),
        ("bottom exactly at the limit", 1200., 800., [290., 440., 600.], [T, T, T], [None, None, None]),
        // Half a pixel or less is no change: 340.3 + 340 leaves 319.7.
        ("under half a pixel", 1000., 800., [340.3, 340., 150.], [T, T, T], [None, None, None]),
        // Nothing here is a number a window gives, but nothing panics and a
        // number that is not a size is no reason to change a dock.
        ("a width that is not a number", nan, 800., [900., 900., 150.], [T, T, T], [None, None, None]),
        ("a dock that is not a number", 1000., 800., [nan, 440., nan], [T, T, T], [None, None, None]),
        ("a dock of infinite size", 1000., 800., [inf, 440., 150.], [T, T, T], [Some(100.), Some(100.), None]),
    ];
    for (case, width, height, sizes, open, expected) in rows {
        let got = clamped(width, height, sizes, open);
        for side in 0..3 {
            match (got[side], expected[side]) {
                (None, None) => {}
                (Some(got), Some(want)) => assert!(
                    (got - want).abs() < 0.01,
                    "{case}: side {side} is {got}, not {want}"
                ),
                (got, want) => panic!("{case}: side {side} is {got:?}, not {want:?}"),
            }
        }
    }
}
