use crate::{
    Cell, DispatchError, Event, Idle, RlfCause, RrcState, StateKind, Transition, dispatch,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioResult {
    pub name: &'static str,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioStep {
    pub event: &'static str,
    pub before: StateKind,
    pub result: Result<Transition, DispatchError>,
}

type Step = (&'static str, Event);

const SETUP_REQUEST: Step = (
    "RRCSetupRequest",
    Event::RrcSetupRequest { transaction_id: 1 },
);
const SETUP: Step = ("RRCSetup", Event::RrcSetup { c_rnti: 0x1234 });
const SETUP_COMPLETE: Step = ("RRCSetupComplete", Event::RrcSetupComplete);
const RELEASE: Step = ("RRCRelease", Event::RrcRelease { suspend: false });
const SUSPEND: Step = (
    "RRCRelease + suspendConfig",
    Event::RrcRelease { suspend: true },
);
const RESUME_REQUEST: Step = ("RRCResumeRequest", Event::RrcResumeRequest);
const REJECT: Step = ("RRCReject", Event::RrcReject);
const LINK_FAILURE: Step = (
    "RadioLinkFailure",
    Event::RadioLinkFailure {
        cause: RlfCause::OutOfSync,
        serving_cell: initial_cell(),
    },
);
const REESTABLISH_REQUEST: Step = (
    "RRCReestablishmentRequest",
    Event::RrcReestablishmentRequest {
        cell: Cell {
            pci: 43,
            rsrp_dbm: -89,
        },
    },
);

pub fn run_all() -> Vec<ScenarioResult> {
    let connected = || [SETUP_REQUEST, SETUP, SETUP_COMPLETE];
    let resuming = || connected().into_iter().chain([SUSPEND, RESUME_REQUEST]);
    let selecting = || connected().into_iter().chain([LINK_FAILURE]);
    let reestablishing = || selecting().chain([REESTABLISH_REQUEST]);
    vec![
        run_scenario(
            "establish-and-release",
            connected().into_iter().chain([RELEASE]),
        ),
        run_scenario(
            "t300-expiry",
            [SETUP_REQUEST, ("T300Expiry", Event::T300Expiry)],
        ),
        run_scenario("radio-link-failure", selecting()),
        run_scenario("unexpected-event", [RELEASE]),
        run_scenario("setup-rejected", [SETUP_REQUEST, REJECT]),
        run_scenario(
            "suspend-and-resume",
            resuming().chain([
                ("RRCResume", Event::RrcResume),
                ("RRCResumeComplete", Event::RrcResumeComplete),
                RELEASE,
            ]),
        ),
        run_scenario("resume-rejected", resuming().chain([REJECT])),
        run_scenario(
            "resume-timeout",
            resuming().chain([("T319Expiry", Event::T319Expiry)]),
        ),
        run_scenario("resume-fallback", resuming().chain([SETUP, SETUP_COMPLETE])),
        run_scenario(
            "reestablish",
            reestablishing().chain([
                (
                    "RRCReestablishment",
                    Event::RrcReestablishment { c_rnti: 0x5678 },
                ),
                (
                    "RRCReestablishmentComplete",
                    Event::RrcReestablishmentComplete,
                ),
                RELEASE,
            ]),
        ),
        run_scenario(
            "cell-selection-timeout",
            selecting().chain([("T311Expiry", Event::T311Expiry)]),
        ),
        run_scenario(
            "reestablishment-timeout",
            reestablishing().chain([("T301Expiry", Event::T301Expiry)]),
        ),
        run_scenario(
            "reestablishment-fallback",
            reestablishing().chain([SETUP, SETUP_COMPLETE]),
        ),
    ]
}

const fn initial_cell() -> Cell {
    Cell {
        pci: 42,
        rsrp_dbm: -95,
    }
}

fn run_scenario(name: &'static str, events: impl IntoIterator<Item = Step>) -> ScenarioResult {
    let mut state = RrcState::Idle(Idle {
        selected_cell: initial_cell(),
    });
    let events = events.into_iter();
    let mut steps = Vec::with_capacity(events.size_hint().0);

    for (event_description, event) in events {
        let before = state.kind();
        let result = dispatch(state.clone(), event);
        if let Ok(transition) = &result {
            state = transition.state.clone();
        }
        steps.push(ScenarioStep {
            event: event_description,
            before,
            result,
        });
    }

    ScenarioResult { name, steps }
}
