use super::{Generations, SettingsPage, TabId, ViewScope};

#[test]
fn a_stamp_is_current_until_its_scope_moves_on() {
    let mut generations = Generations::default();
    let first = generations.next(ViewScope::Repositories);
    assert!(generations.is_current(&first));

    let second = generations.next(ViewScope::Repositories);
    assert!(!generations.is_current(&first));
    assert!(generations.is_current(&second));
    assert_ne!(first.generation, second.generation);
}

#[test]
fn moving_one_scope_on_leaves_the_others_current() {
    let mut generations = Generations::default();
    let documents = generations.next(ViewScope::Documents("/a".to_owned()));
    let other_documents = generations.next(ViewScope::Documents("/b".to_owned()));
    let tickets = generations.next(ViewScope::Tickets("/a".to_owned()));
    let tab = generations.next(ViewScope::Tab(TabId(1)));
    let keys = generations.next(ViewScope::Settings(SettingsPage::Keys));

    generations.next(ViewScope::Documents("/a".to_owned()));

    assert!(!generations.is_current(&documents));
    for stamp in [&other_documents, &tickets, &tab, &keys] {
        assert!(generations.is_current(stamp), "{stamp:?}");
    }
}

#[test]
fn a_stamp_never_issued_is_not_current() {
    let generations = Generations::default();
    let stamp = super::Stamp {
        scope: ViewScope::Operations("/a".to_owned()),
        generation: 0,
    };
    assert!(!generations.is_current(&stamp));
}
