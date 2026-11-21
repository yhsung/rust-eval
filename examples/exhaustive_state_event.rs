#[derive(Clone, Copy)]
enum State {
    Idle,
    Connecting,
    Connected,
}

#[derive(Clone, Copy)]
enum Event {
    StartConnection,
    RrcSetup,
    T300Expiry,
    Release,
}

fn coverage_token(state: State, event: Event) -> u8 {
    match (state, event) {
        (State::Idle, Event::StartConnection) => 0,
        (State::Idle, Event::RrcSetup) => 1,
        (State::Idle, Event::T300Expiry) => 2,
        (State::Idle, Event::Release) => 3,
        (State::Connecting, Event::StartConnection) => 4,
        (State::Connecting, Event::RrcSetup) => 5,
        (State::Connecting, Event::T300Expiry) => 6,
        (State::Connecting, Event::Release) => 7,
        (State::Connected, Event::StartConnection) => 8,
        (State::Connected, Event::RrcSetup) => 9,
        (State::Connected, Event::T300Expiry) => 10,
        (State::Connected, Event::Release) => 11,
    }
}

fn main() {
    let states = [State::Idle, State::Connecting, State::Connected];
    let events = [
        Event::StartConnection,
        Event::RrcSetup,
        Event::T300Expiry,
        Event::Release,
    ];

    let total: u8 = states
        .into_iter()
        .flat_map(|state| {
            events
                .into_iter()
                .map(move |event| coverage_token(state, event))
        })
        .sum();
    assert_eq!(total, 66);
}
