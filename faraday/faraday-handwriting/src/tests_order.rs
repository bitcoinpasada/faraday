//! Tests of the reading order and of the form chosen, on synthetic boxes: no
//! network and no image.

use crate::find::Rect;
use crate::order::{Group, Item, arrange, pair_numbers};
use crate::page::{FIT_FLOOR, Form, Question, Row, bip39_counts, choose_form, slip39_counts};

const H: f32 = 20.0;
const PITCH: f32 = 40.0;
const WIDTH: f32 = 100.0;

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect { x0, y0, x1, y1 }
}

/// A word box starting at `x`, line `row` (pitch 40, height 20), at a
/// vertical offset.
fn at(x: f32, row: f32, y_off: f32) -> Rect {
    let y0 = y_off + row * PITCH;
    rect(x, y0, x + WIDTH, y0 + H)
}

fn item(rect: Rect, value: Option<u8>) -> Item {
    Item { rect, value }
}

/// `n` words down one column at `x`, numbered from `first` when given.
fn column(x: f32, n: usize, y_off: f32, first: Option<u8>) -> Vec<Item> {
    (0..n)
        .map(|i| item(at(x, i as f32, y_off), first.map(|f| f + i as u8)))
        .collect()
}

fn ids(g: &Group) -> Vec<usize> {
    g.order.clone()
}

#[test]
fn one_column_is_one_list_top_to_bottom_whatever_the_input_order() {
    let mut items = column(10.0, 12, 0.0, None);
    items.reverse();
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].across, None);
    assert!(!g[0].numbers_disagree);
    let want: Vec<usize> = (0..12).rev().collect();
    assert_eq!(ids(&g[0]), want);
}

#[test]
fn no_boxes_no_lists() {
    assert!(arrange(&[]).is_empty());
}

#[test]
fn two_columns_without_numbers_give_both_orders() {
    let mut items = column(10.0, 6, 0.0, None);
    items.extend(column(300.0, 6, 0.0, None));
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
    let across: Vec<usize> = vec![0, 6, 1, 7, 2, 8, 3, 9, 4, 10, 5, 11];
    assert_eq!(g[0].across, Some(across));
}

#[test]
fn two_columns_with_numbers_down_or_across_follow_the_numbers() {
    let mut down = column(10.0, 6, 0.0, Some(1));
    down.extend(column(300.0, 6, 0.0, Some(7)));
    let g = arrange(&down);
    assert_eq!(g.len(), 1);
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
    assert_eq!(g[0].across, None);
    // Across the rows: the left column holds 1, 3, 5, ...
    let mut rows = Vec::new();
    for i in 0..6u8 {
        rows.push(item(at(10.0, f32::from(i), 0.0), Some(2 * i + 1)));
    }
    for i in 0..6u8 {
        rows.push(item(at(300.0, f32::from(i), 0.0), Some(2 * i + 2)));
    }
    let g = arrange(&rows);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].across, None);
    let want: Vec<usize> = vec![0, 6, 1, 7, 2, 8, 3, 9, 4, 10, 5, 11];
    assert_eq!(ids(&g[0]), want);
}

#[test]
fn tight_columns_with_a_small_gap_are_two_columns() {
    let mut items = column(10.0, 6, 0.0, None);
    items.extend(column(10.0 + WIDTH + 2.0, 6, 0.0, None));
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].across.is_some());
}

#[test]
fn columns_whose_words_overlap_a_little_are_not_read_as_one_column() {
    // Words of different widths: the left column's longest word runs 10
    // pixels into the right column. Two words stand on every line, so this is
    // not one column read top to bottom without a question.
    let mut items = Vec::new();
    for i in 0..6 {
        let y = i as f32 * PITCH;
        let wide = if i == 3 { 130.0 } else { 90.0 };
        items.push(item(rect(10.0, y, 10.0 + wide, y + H), None));
    }
    for i in 0..6 {
        let y = i as f32 * PITCH;
        items.push(item(rect(130.0, y, 230.0, y + H), None));
    }
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].across.is_some(), "two words on a line, no question");
}

#[test]
fn a_ragged_last_row_keeps_every_word_in_both_orders() {
    let mut items = column(10.0, 6, 0.0, None);
    items.extend(column(300.0, 5, 0.0, None));
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert_eq!(ids(&g[0]), (0..11).collect::<Vec<_>>());
    let across = g[0].across.clone().unwrap();
    assert_eq!(across, vec![0, 6, 1, 7, 2, 8, 3, 9, 4, 10, 5]);
}

#[test]
fn a_missing_number_in_one_column_leaves_the_order_by_position() {
    let mut items = column(10.0, 12, 0.0, Some(1));
    items[4].value = None;
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
    assert!(!g[0].numbers_disagree);
}

#[test]
fn a_missing_number_in_two_columns_is_an_order_question() {
    let mut items = column(10.0, 6, 0.0, Some(1));
    items.extend(column(300.0, 6, 0.0, Some(7)));
    items[2].value = None;
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].across.is_some());
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
}

#[test]
fn numbers_that_repeat_are_reported_in_one_column() {
    let mut items = column(10.0, 12, 0.0, Some(1));
    items[6].value = Some(5);
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].numbers_disagree);
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
}

#[test]
fn numbers_that_conflict_in_two_columns_are_reported() {
    let mut items = column(10.0, 6, 0.0, Some(1));
    items.extend(column(300.0, 6, 0.0, Some(1)));
    // 1 to 6 twice: two lists of six by their numbers.
    let g = arrange(&items);
    assert_eq!(g.len(), 2);
    // With numbers that fit no list, nothing numbers a list: the question is
    // asked and the numbers are reported.
    items[8].value = Some(30);
    items[9].value = Some(2);
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].across.is_some());
    assert!(g[0].numbers_disagree);
}

#[test]
fn numbering_that_starts_again_makes_two_lists_in_one_column() {
    // No gap between the two lists; only the numbers say where one ends.
    let mut items = column(10.0, 12, 0.0, Some(1));
    items.extend(column(10.0, 12, 12.0 * PITCH, Some(1)));
    let g = arrange(&items);
    assert_eq!(g.len(), 2);
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
    assert_eq!(ids(&g[1]), (12..24).collect::<Vec<_>>());
}

#[test]
fn a_vertical_gap_cutting_lists_that_fit_one_form_whole_is_flagged() {
    let mut items = column(10.0, 12, 0.0, None);
    items.extend(column(10.0, 12, 12.0 * PITCH + 200.0, None));
    let g = arrange(&items);
    // Two lists of 12 or one of 24: the lists are cut, and each is flagged so
    // that the page asks.
    assert_eq!(g.len(), 2);
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
    assert_eq!(ids(&g[1]), (12..24).collect::<Vec<_>>());
    assert!(g.iter().all(|x| x.cut_by_gap));
}

#[test]
fn one_column_with_no_gap_is_not_flagged_as_cut() {
    let g = arrange(&column(10.0, 24, 0.0, None));
    assert_eq!(g.len(), 1);
    assert!(!g[0].cut_by_gap);
}

#[test]
fn two_lists_side_by_side_without_numbers_name_their_columns() {
    let mut items = column(10.0, 12, 0.0, None);
    items.extend(column(300.0, 12, 0.0, None));
    let g = arrange(&items);
    // One list of 24 or two of 12: both orders are offered and the sizes of
    // the columns are given so that the page asks.
    assert_eq!(g.len(), 1);
    assert!(g[0].across.is_some());
    assert_eq!(g[0].order.len(), 24);
    assert_eq!(g[0].sides, vec![12, 12]);
}

#[test]
fn two_numbered_lists_side_by_side_are_two_lists() {
    let mut items = column(10.0, 12, 0.0, Some(1));
    items.extend(column(300.0, 12, 0.0, Some(1)));
    let g = arrange(&items);
    assert_eq!(g.len(), 2);
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
    assert_eq!(ids(&g[1]), (12..24).collect::<Vec<_>>());
}

#[test]
fn numbers_read_beside_the_second_half_of_a_phrase_are_asked_about() {
    let g = arrange(&column(10.0, 12, 0.0, Some(13)));
    assert_eq!(g.len(), 1);
    assert!(g[0].numbers_disagree);
}

#[test]
fn a_word_not_found_in_a_numbered_column_is_a_numbers_question() {
    // 28 words, the seventh not found: numbers 6 then 8.
    let mut items = column(10.0, 28, 0.0, Some(1));
    items.remove(6);
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].numbers_disagree);
}

#[test]
fn a_restart_that_the_numbers_do_not_prove_does_not_cut_the_list() {
    // 24 words, the thirteenth read as 1 although the 3 were lost: not two
    // lists of 12.
    let mut items = column(10.0, 24, 0.0, Some(1));
    items[12].value = Some(1);
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].order.len(), 24);
    assert!(g[0].numbers_disagree);
}

#[test]
fn a_single_column_is_never_silently_reordered_by_its_numbers() {
    let mut items = column(10.0, 12, 0.0, Some(1));
    items[6].value = Some(9);
    items[8].value = Some(7);
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].numbers_disagree);
}

#[test]
fn numbers_that_fall_down_two_columns_are_judged_on_the_order_returned() {
    // Numbered across the rows with one number unread: the order down does
    // not match the numbers, the order across does, and that one comes first.
    let mut items = Vec::new();
    for i in 0..6u8 {
        items.push(item(at(10.0, f32::from(i), 0.0), Some(2 * i + 1)));
    }
    for i in 0..6u8 {
        items.push(item(at(300.0, f32::from(i), 0.0), Some(2 * i + 2)));
    }
    items[3].value = None;
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(!g[0].numbers_disagree);
    assert_eq!(ids(&g[0]), vec![0, 6, 1, 7, 2, 8, 3, 9, 4, 10, 5, 11]);
    // With nothing that fits either order, the numbers are reported.
    items[0].value = Some(40);
    let g = arrange(&items);
    assert!(g[0].numbers_disagree);
}

#[test]
fn plainly_numbered_pages_are_certain() {
    for n in [12usize, 18, 24] {
        let g = arrange(&column(10.0, n, 0.0, Some(1)));
        assert_eq!(g.len(), 1, "{n}");
        assert!(!g[0].numbers_disagree && g[0].across.is_none(), "{n}");
        let g = arrange(&column(10.0, n, 0.0, None));
        assert_eq!(g.len(), 1, "{n}");
        assert!(!g[0].numbers_disagree && g[0].across.is_none(), "{n}");
        assert!(!g[0].cut_by_gap, "{n}");
        for cols in [2usize, 3] {
            let per = n / cols;
            // Numbers down the columns.
            let mut down = Vec::new();
            for c in 0..cols {
                down.extend(column(
                    10.0 + 300.0 * c as f32,
                    per,
                    0.0,
                    Some((c * per + 1) as u8),
                ));
            }
            let g = arrange(&down);
            assert_eq!(g.len(), 1, "{n} {cols}");
            assert_eq!(ids(&g[0]), (0..n).collect::<Vec<_>>());
            assert!(
                g[0].across.is_none() && !g[0].numbers_disagree,
                "{n} {cols}"
            );
            // Numbers across the rows.
            let mut rows = Vec::new();
            for c in 0..cols {
                for r in 0..per {
                    rows.push(item(
                        at(10.0 + 300.0 * c as f32, r as f32, 0.0),
                        Some((r * cols + c + 1) as u8),
                    ));
                }
            }
            let g = arrange(&rows);
            assert_eq!(g.len(), 1, "{n} {cols}");
            assert!(
                g[0].across.is_none() && !g[0].numbers_disagree,
                "{n} {cols}"
            );
            let numbers: Vec<u8> = g[0].order.iter().filter_map(|&i| rows[i].value).collect();
            assert_eq!(numbers, (1..=n as u8).collect::<Vec<_>>(), "{n} {cols}");
        }
    }
    for n in [20usize, 33] {
        let g = arrange(&column(10.0, n, 0.0, Some(1)));
        assert_eq!(g.len(), 1, "{n}");
        assert!(!g[0].numbers_disagree && g[0].across.is_none(), "{n}");
        let per = n.div_ceil(2);
        let mut two = column(10.0, per, 0.0, Some(1));
        two.extend(column(300.0, n - per, 0.0, Some(per as u8 + 1)));
        let g = arrange(&two);
        assert_eq!(g.len(), 1, "{n}");
        assert!(g[0].across.is_none() && !g[0].numbers_disagree, "{n}");
    }
}

#[test]
fn two_columns_with_the_right_one_half_a_line_lower_are_not_one_column() {
    // The left column holds one long word that reaches under the right
    // column's left edge.
    let mut items = Vec::new();
    for i in 0..12 {
        let y = i as f32 * PITCH;
        let wide = if i == 3 { 130.0 } else { 90.0 };
        items.push(item(rect(10.0, y, 10.0 + wide, y + H), None));
    }
    for i in 0..12 {
        let y = i as f32 * PITCH + 20.0;
        items.push(item(rect(130.0, y, 230.0, y + H), None));
    }
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].across.is_some(), "read as one column with no question");
    assert_eq!(g[0].sides, vec![12, 12]);
}

/// Two unnumbered columns on a page slanted by `degrees`: 12 words each, the
/// columns 270 apart, a line pitch of 58.
fn slanted(degrees: f32, wide: f32) -> Vec<Item> {
    let slope = degrees.to_radians().tan();
    let mut items = Vec::new();
    for c in 0..2 {
        for i in 0..12 {
            let y = i as f32 * 58.0 + 6.0;
            let x = 10.0 + 270.0 * c as f32 + slope * y;
            items.push(item(rect(x, y, x + wide, y + H), None));
        }
    }
    items
}

#[test]
fn slanted_columns_are_asked_about_both_ways() {
    for degrees in [-8.0f32, -5.0, 5.0, 8.0] {
        for wide in [100.0f32, 200.0, 260.0] {
            let g = arrange(&slanted(degrees, wide));
            assert_eq!(g.len(), 1, "{degrees} {wide}");
            assert!(g[0].across.is_some(), "{degrees} {wide}: no question");
            assert_eq!(g[0].order.len(), 24);
        }
    }
}

#[test]
fn a_slanted_single_column_is_still_one_column() {
    let mut items = Vec::new();
    for i in 0..12 {
        let y = i as f32 * 58.0;
        let x = 10.0 + 8.0f32.to_radians().tan() * y;
        items.push(item(rect(x, y, x + 100.0, y + H), None));
    }
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].across.is_none());
    assert_eq!(ids(&g[0]), (0..12).collect::<Vec<_>>());
}

#[test]
fn boxes_that_are_not_finite_do_not_panic() {
    let bad = rect(f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::NAN);
    let mut items = column(10.0, 5, 0.0, Some(1));
    items.push(item(bad, Some(3)));
    items.push(item(bad, None));
    let _ = arrange(&items);
    let _ = pair_numbers(&[bad, at(0.0, 0.0, 0.0)], &[bad, rect(0.0, 0.0, 5.0, 5.0)]);
}

// --- Numbers beside words ---------------------------------------------------

#[test]
fn a_number_goes_with_the_word_to_its_right() {
    let words = [at(60.0, 0.0, 0.0), at(60.0, 1.0, 0.0)];
    let numbers = [rect(10.0, 0.0, 40.0, H), rect(10.0, PITCH, 40.0, PITCH + H)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![Some(0), Some(1)]);
    assert_eq!(unattached, 0);
}

#[test]
fn a_number_far_from_a_word_or_on_another_line_goes_with_none() {
    let words = [at(500.0, 0.0, 0.0)];
    let numbers = [rect(10.0, 0.0, 40.0, H), rect(450.0, 200.0, 480.0, 220.0)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![None]);
    assert_eq!(unattached, 2);
}

#[test]
fn a_number_right_of_a_word_is_not_its_line_number() {
    let words = [at(10.0, 0.0, 0.0)];
    let numbers = [rect(130.0, 0.0, 160.0, H)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![None]);
    assert_eq!(unattached, 1);
}

#[test]
fn a_number_between_two_columns_goes_with_the_word_on_its_right() {
    let words = [at(10.0, 0.0, 0.0), at(300.0, 0.0, 0.0)];
    let numbers = [rect(260.0, 0.0, 290.0, H)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![None, Some(0)]);
    assert_eq!(unattached, 0);
}

#[test]
fn two_numbers_for_one_word_leave_the_farther_one_over() {
    let words = [at(100.0, 0.0, 0.0)];
    let numbers = [rect(10.0, 0.0, 40.0, H), rect(60.0, 0.0, 90.0, H)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![Some(1)]);
    assert_eq!(unattached, 1);
}

#[test]
fn no_boxes_pair_nothing() {
    assert_eq!(pair_numbers(&[], &[]), (vec![], 0));
    assert_eq!(pair_numbers(&[at(0.0, 0.0, 0.0)], &[]), (vec![None], 0));
    assert_eq!(pair_numbers(&[], &[rect(0.0, 0.0, 5.0, 5.0)]), (vec![], 1));
}

#[test]
fn a_number_goes_with_the_word_on_its_line_not_the_one_starting_nearer() {
    // The number stands on the second line; the first line's word starts
    // nearer along x.
    let words = [rect(60.0, 0.0, 160.0, 20.0), rect(70.0, 24.0, 170.0, 44.0)];
    let numbers = [rect(10.0, 26.0, 40.0, 46.0)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![None, Some(0)]);
    assert_eq!(unattached, 0);
}

#[test]
fn a_number_about_as_near_two_words_goes_with_neither() {
    let words = [rect(60.0, 0.0, 160.0, 20.0), rect(60.0, 20.0, 160.0, 40.0)];
    let numbers = [rect(10.0, 10.0, 40.0, 30.0)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![None, None]);
    assert_eq!(unattached, 1);
}

#[test]
fn a_number_that_loses_its_word_takes_its_next_word_on_its_line() {
    // The first number is nearer the first word; the second number is on the
    // same line, further left, and takes the word beside it.
    let words = [rect(100.0, 0.0, 140.0, 40.0), rect(150.0, 0.0, 190.0, 40.0)];
    let numbers = [rect(60.0, 0.0, 90.0, 40.0), rect(10.0, 0.0, 40.0, 40.0)];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![Some(0), Some(1)]);
    assert_eq!(unattached, 0);
}

// --- Count and form ---------------------------------------------------------

fn table(extra: Option<&[usize]>) -> Vec<Row> {
    let mut t = vec![
        Row {
            form: Form::Bip39,
            list: 0,
            counts: bip39_counts(),
        },
        Row {
            form: Form::Slip39,
            list: 1,
            counts: slip39_counts(),
        },
    ];
    if let Some(c) = extra {
        t.push(Row {
            form: Form::Extra,
            list: 2,
            counts: c.to_vec(),
        });
    }
    t
}

#[test]
fn every_bip39_count_chooses_bip39_when_its_list_fits() {
    assert_eq!(bip39_counts(), vec![12, 15, 18, 21, 24]);
    for n in bip39_counts() {
        assert_eq!(
            choose_form(n, &[-0.5, -3.0], &table(None)),
            Ok((Form::Bip39, 0)),
            "{n}"
        );
    }
}

#[test]
fn every_slip39_count_chooses_slip39_when_its_list_fits() {
    assert_eq!(slip39_counts(), vec![20, 22, 23, 25, 27, 28, 30, 31, 33]);
    for n in slip39_counts() {
        assert_eq!(
            choose_form(n, &[-3.0, -0.5], &table(None)),
            Ok((Form::Slip39, 1)),
            "{n}"
        );
    }
}

#[test]
fn a_count_that_fits_no_form_is_a_count_question() {
    for n in [0, 1, 11, 13, 14, 16, 17, 19, 26, 29, 32, 34, 100] {
        assert_eq!(
            choose_form(n, &[-0.5, -0.5], &table(None)),
            Err(Question::Count),
            "{n}"
        );
    }
}

#[test]
fn a_count_and_a_list_that_disagree_is_a_form_question() {
    assert_eq!(
        choose_form(12, &[-9.0, -0.5], &table(None)),
        Err(Question::Form)
    );
    assert_eq!(
        choose_form(23, &[-0.5, -9.0], &table(None)),
        Err(Question::Form)
    );
}

#[test]
fn a_small_difference_between_lists_is_not_a_question() {
    assert_eq!(
        choose_form(12, &[-1.0, -0.5], &table(None)),
        Ok((Form::Bip39, 0))
    );
}

#[test]
fn words_with_no_reading_in_any_list_are_a_form_question_not_a_count_guess() {
    assert_eq!(
        choose_form(12, &[-1e9, -1e9], &table(None)),
        Err(Question::Form)
    );
}

#[test]
fn a_list_the_words_do_not_fit_is_not_chosen_by_count() {
    // Under the floor in both lists: asked. Above it: chosen.
    assert_eq!(
        choose_form(12, &[FIT_FLOOR - 1.0, FIT_FLOOR - 2.0], &table(None)),
        Err(Question::Form)
    );
    assert_eq!(
        choose_form(12, &[FIT_FLOOR + 1.0, FIT_FLOOR - 2.0], &table(None)),
        Ok((Form::Bip39, 0))
    );
}

#[test]
fn two_forms_with_one_count_and_alike_lists_are_a_form_question() {
    let t = table(Some(&[12, 36]));
    assert_eq!(
        choose_form(12, &[-0.5, -2.0, -0.6], &t),
        Err(Question::Form)
    );
}

#[test]
fn the_form_whose_list_fits_much_better_wins_a_shared_count() {
    let t = table(Some(&[12, 36]));
    assert_eq!(
        choose_form(12, &[-5.0, -6.0, -0.5], &t),
        Ok((Form::Extra, 2))
    );
    assert_eq!(
        choose_form(12, &[-0.5, -6.0, -5.0], &t),
        Ok((Form::Bip39, 0))
    );
}

#[test]
fn an_extra_list_of_36_48_and_56_words() {
    let t = table(Some(&[36, 48, 56]));
    for n in [36, 48, 56] {
        assert_eq!(
            choose_form(n, &[-5.0, -5.0, -0.5], &t),
            Ok((Form::Extra, 2)),
            "{n}"
        );
        // The words fit another list better than the form's: asked about.
        assert_eq!(
            choose_form(n, &[-0.5, -5.0, -5.0], &t),
            Err(Question::Form),
            "{n}"
        );
    }
    assert_eq!(
        choose_form(40, &[-0.5, -0.5, -0.5], &t),
        Err(Question::Count)
    );
}

#[test]
fn two_numbers_that_both_lost_and_want_one_free_word_leave_it_to_neither() {
    // Two tight lines; three numbers on the first line, the nearest keeps it.
    let words = [rect(100.0, 0.0, 200.0, 20.0), rect(100.0, 8.0, 200.0, 28.0)];
    let numbers = [
        rect(60.0, 0.0, 90.0, 20.0),
        rect(30.0, 0.0, 60.0, 20.0),
        rect(20.0, 0.0, 50.0, 20.0),
    ];
    let (p, unattached) = pair_numbers(&words, &numbers);
    assert_eq!(p, vec![Some(0), None]);
    assert_eq!(unattached, 2);
    // One that lost alone takes the free word.
    let (p, unattached) = pair_numbers(&words, &numbers[..2]);
    assert_eq!(p, vec![Some(0), Some(1)]);
    assert_eq!(unattached, 0);
}

// --- A number between two lines, numbers across, bands, an unclear layout ------

#[test]
fn a_number_between_two_lines_goes_with_neither() {
    // Two tight lines, the second a line lower and a little to the right; a
    // number as far from both centres: either would be a guess.
    let words = [rect(60.0, 0.0, 160.0, 20.0), rect(70.0, 20.0, 170.0, 40.0)];
    let (p, unattached) = pair_numbers(&words, &[rect(10.0, 10.0, 40.0, 30.0)]);
    assert_eq!(p, vec![None, None]);
    assert_eq!(unattached, 1);
    // A little nearer one line than the other: it goes with that one.
    let (p, unattached) = pair_numbers(&words, &[rect(10.0, 4.0, 40.0, 24.0)]);
    assert_eq!(p, vec![Some(0), None]);
    assert_eq!(unattached, 0);
}

#[test]
fn numbers_that_run_across_the_columns_put_the_rows_first() {
    let mut down = column(10.0, 6, 0.0, Some(1));
    down.extend(column(300.0, 6, 0.0, Some(7)));
    assert!(!arrange(&down)[0].rows_first);
    // One word's number not read, so the numbers cannot cut lists: they say
    // the rows come first.
    let mut rows = Vec::new();
    for i in 0..6u8 {
        let first = if i == 0 { None } else { Some(2 * i + 1) };
        rows.push(item(at(10.0, f32::from(i), 0.0), first));
        rows.push(item(at(300.0, f32::from(i), 0.0), Some(2 * i + 2)));
    }
    let g = arrange(&rows);
    assert!(g[0].rows_first);
    assert_eq!(g[0].order, (0..12).collect::<Vec<_>>());
    // Unnumbered, the down order comes first.
    let mut plain = column(10.0, 6, 0.0, None);
    plain.extend(column(300.0, 6, 0.0, None));
    assert!(!arrange(&plain)[0].rows_first);
}

#[test]
fn a_blank_band_across_every_column_is_named_and_one_across_some_is_not() {
    // Two columns of six, a blank band of 5 pitches across both after the
    // third line.
    let mut items = Vec::new();
    for i in 0..6 {
        let low = if i >= 3 { 5.0 * PITCH } else { 0.0 };
        items.push(item(at(10.0, i as f32, low), None));
        items.push(item(at(300.0, i as f32, low), None));
    }
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].sides, vec![6, 6]);
    assert_eq!(g[0].bands, vec![6, 6]);
    // The left column has the same gap, the right one runs on through it:
    // no band across them all.
    let mut items = Vec::new();
    for i in 0..6 {
        let low = if i >= 3 { 5.0 * PITCH } else { 0.0 };
        items.push(item(at(10.0, i as f32, low), None));
    }
    items.extend(column(300.0, 11, 0.0, None));
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].sides, vec![6, 11]);
    assert!(g[0].bands.is_empty());
}

#[test]
fn boxes_that_are_neither_one_column_nor_several_are_an_unclear_layout() {
    // Two words on every line, each pair a little further right than the line
    // above: no column holds them, and no cut makes columns.
    let mut items = Vec::new();
    for i in 0..8 {
        let (y, x) = (i as f32 * PITCH, 20.0 * i as f32);
        items.push(item(rect(10.0 + x, y, 110.0 + x, y + H), None));
        items.push(item(rect(70.0 + x, y, 170.0 + x, y + H), None));
    }
    let g = arrange(&items);
    assert_eq!(g.len(), 1);
    assert!(g[0].unclear, "{:?}", g[0]);
    // A plain page is never unclear.
    assert!(!arrange(&column(10.0, 12, 0.0, None))[0].unclear);
}
