use super::*;

#[test]
fn legacy_hrw_inputs_and_first_candidate_tie_break_are_stable() {
    assert_eq!(score(&[b"account", &[1, 2, 3]]), 0x4a76311702647b46);
    assert_eq!(
        score(&[b"users", &[0xff], b"account", &17_u32.to_le_bytes()]),
        0xd3fb99f94b50f9da
    );
    let mut best = None;
    select(&mut best, 0, ChildId::SECOND);
    select(&mut best, 0, ChildId::FIRST);
    assert_eq!(best, Some((0, ChildId::SECOND)));
}
#[test]
fn replacement_never_resolves_an_original_account() {
    let original = Allocation {
        slot: 1,
        generation: [42; 32],
    };
    for (pid, generation) in [("original", [41; 32]), ("replacement", [42; 32])] {
        let current = Status {
            phase: Phase::Ready,
            worker: Some(pid.into()),
            ..Default::default()
        };
        assert_eq!(
            resolve(
                original.clone(),
                Some("original"),
                Some(generation),
                &current
            ),
            Placement::Unavailable(original.clone(), Some("original".into()))
        );
    }
}

#[test]
fn uncertain_dispatch_retains_the_original_pending_allocation() {
    let original = Allocation {
        slot: 1,
        generation: [7; 32],
    };
    let status = Status {
        phase: Phase::CreateIssued,
        failure: Some(Failure::Uncertain),
        ..Default::default()
    };
    assert_eq!(
        resolve(original.clone(), None, Some(original.generation), &status),
        Placement::Pending(original)
    );
}
