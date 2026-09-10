use rust_modem_safety_island::{Cell, Event, EventKind, RlfCause, Timer, TimerId};

#[test]
fn typed_events_keep_their_payload_and_kind() {
    let event = Event::RadioLinkFailure {
        cause: RlfCause::OutOfSync,
        serving_cell: Cell {
            pci: 123,
            rsrp_dbm: -85,
        },
    };

    assert_eq!(event.kind(), EventKind::RadioLinkFailure);
    assert_eq!(Timer::start(TimerId::T300).is_running, true);
    assert_eq!(
        format!("{event:?}"),
        "RadioLinkFailure { cause: OutOfSync, serving_cell: Cell { pci: 123, rsrp_dbm: -85 } }"
    );
}
