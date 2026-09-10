use crate::{Cell, DispatchError, Event, Idle, RlfCause, RrcState, Transition, dispatch};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioResult {
    pub name: &'static str,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioStep {
    pub event: &'static str,
    pub result: Result<Transition, DispatchError>,
}

pub fn run_all() -> Vec<ScenarioResult> {
    vec![
        run_scenario(
            "establish-and-release",
            vec![
                (
                    "start connection",
                    Event::StartConnection { transaction_id: 1 },
                ),
                (
                    "receive setup complete",
                    Event::SetupComplete { c_rnti: 0x1234 },
                ),
                ("receive release", Event::Release),
            ],
        ),
        run_scenario(
            "t300-expiry",
            vec![
                (
                    "start connection",
                    Event::StartConnection { transaction_id: 2 },
                ),
                ("T300 expires", Event::T300Expiry),
            ],
        ),
        run_scenario(
            "radio-link-failure",
            vec![
                (
                    "start connection",
                    Event::StartConnection { transaction_id: 3 },
                ),
                (
                    "receive setup complete",
                    Event::SetupComplete { c_rnti: 0x5678 },
                ),
                (
                    "report radio link failure",
                    Event::RadioLinkFailure {
                        cause: RlfCause::OutOfSync,
                        serving_cell: initial_cell(),
                    },
                ),
            ],
        ),
        run_scenario(
            "unexpected-event",
            vec![("receive release", Event::Release)],
        ),
    ]
}

fn initial_cell() -> Cell {
    Cell {
        pci: 42,
        rsrp_dbm: -95,
    }
}

fn run_scenario(name: &'static str, events: Vec<(&'static str, Event)>) -> ScenarioResult {
    let mut state = RrcState::Idle(Idle {
        selected_cell: initial_cell(),
    });
    let mut steps = Vec::with_capacity(events.len());

    for (event_description, event) in events {
        let result = dispatch(state.clone(), event);
        if let Ok(transition) = &result {
            state = transition.state.clone();
        }
        steps.push(ScenarioStep {
            event: event_description,
            result,
        });
    }

    ScenarioResult { name, steps }
}
