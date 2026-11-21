use crate::{
    Cell, Completion, Event, Idle, RlfCause, RrcState, ScenarioResult, ScenarioStep,
    T300_VALUES_MS, dispatch,
};

const MAX_EVENTS: usize = 24;
const T301_MS: u64 = 100;
const T311_MS: u64 = 1000;
const T319_MS: u64 = 100;

#[derive(Debug, PartialEq, Eq)]
pub struct StochasticRun {
    pub scenario: ScenarioResult,
    pub steps_ms: Vec<u64>,
}

/// Generates one deterministic trace using a simulated clock, not OS timers.
///
/// Panics if `t300_ms` is not a member of [`T300_VALUES_MS`].
pub fn stochastic_run(seed: u64, t300_ms: u64) -> StochasticRun {
    assert!(T300_VALUES_MS.contains(&t300_ms));
    let mut rng = Draw(seed);
    let mut state = RrcState::Idle(Idle {
        selected_cell: Cell {
            pci: 42,
            rsrp_dbm: -95,
        },
    });
    let mut steps = Vec::with_capacity(MAX_EVENTS);
    let mut steps_ms = Vec::with_capacity(MAX_EVENTS);
    let mut clock_ms = 0;
    for i in 0..MAX_EVENTS {
        let (delay, label, event) = if i == 0 {
            (
                0,
                "RRCSetupRequest",
                Event::RrcSetupRequest { transaction_id: 1 },
            )
        } else {
            next(&state, &mut rng, t300_ms)
        };
        let before = state.kind();
        let transition = dispatch(state, event).expect("generator emits valid events");
        state = transition.state.clone();
        steps.push(ScenarioStep {
            event: label,
            before,
            cm_before: before.cm_state(),
            result: Ok(transition),
        });
        clock_ms += delay;
        steps_ms.push(clock_ms);
        if matches!(state, RrcState::Idle(_)) {
            break;
        }
    }
    StochasticRun {
        scenario: ScenarioResult {
            name: "Stochastic Run",
            steps,
        },
        steps_ms,
    }
}

type Scheduled = (u64, &'static str, Event);

fn next(state: &RrcState, rng: &mut Draw, t300_ms: u64) -> Scheduled {
    let c_rnti = 0x1234;
    match state {
        RrcState::Idle(_) => unreachable!("Idle is terminal after the initial request"),
        RrcState::Connecting(_) => {
            let reply = if rng.below(5) == 0 {
                ("RRCReject", Event::RrcReject)
            } else {
                ("RRCSetup", Event::RrcSetup { c_rnti })
            };
            timed(rng, t300_ms, reply, ("T300Expiry", Event::T300Expiry))
        }
        RrcState::Connected(link) => {
            let (label, event) = match link.pending_completion {
                Some(Completion::Setup) => return (1, "RRCSetupComplete", Event::RrcSetupComplete),
                Some(Completion::Resume) => {
                    return (1, "RRCResumeComplete", Event::RrcResumeComplete);
                }
                Some(Completion::Reestablishment) => {
                    return (
                        1,
                        "RRCReestablishmentComplete",
                        Event::RrcReestablishmentComplete,
                    );
                }
                None => match rng.below(3) {
                    0 => ("RRCRelease", Event::RrcRelease { suspend: false }),
                    1 => (
                        "RRCRelease + suspendConfig",
                        Event::RrcRelease { suspend: true },
                    ),
                    _ => (
                        "RadioLinkFailure",
                        Event::RadioLinkFailure {
                            cause: RlfCause::OutOfSync,
                            serving_cell: link.serving_cell.clone(),
                        },
                    ),
                },
            };
            (20 + rng.below(181), label, event)
        }
        RrcState::Inactive(_) => (
            20 + rng.below(181),
            "RRCResumeRequest",
            Event::RrcResumeRequest,
        ),
        RrcState::Resuming(_) => {
            let reply = match rng.below(5) {
                0 => ("RRCReject", Event::RrcReject),
                1 => ("RRCSetup", Event::RrcSetup { c_rnti }),
                _ => ("RRCResume", Event::RrcResume),
            };
            timed(rng, T319_MS, reply, ("T319Expiry", Event::T319Expiry))
        }
        RrcState::SelectingCell(procedure) => timed(
            rng,
            T311_MS,
            (
                "RRCReestablishmentRequest",
                Event::RrcReestablishmentRequest {
                    cell: Cell {
                        pci: (procedure.previous_cell.pci + 1) % 1008,
                        rsrp_dbm: -89,
                    },
                },
            ),
            ("T311Expiry", Event::T311Expiry),
        ),
        RrcState::Reestablishing(_) => {
            let reply = if rng.below(5) == 0 {
                ("RRCSetup", Event::RrcSetup { c_rnti })
            } else {
                ("RRCReestablishment", Event::RrcReestablishment { c_rnti })
            };
            timed(rng, T301_MS, reply, ("T301Expiry", Event::T301Expiry))
        }
    }
}

fn timed(
    rng: &mut Draw,
    deadline_ms: u64,
    response: (&'static str, Event),
    expiry: (&'static str, Event),
) -> Scheduled {
    let delay = deadline_ms / 2 + rng.below(deadline_ms + 1);
    if delay < deadline_ms {
        (delay, response.0, response.1)
    } else {
        (deadline_ms, expiry.0, expiry.1)
    }
}

struct Draw(u64);

impl Draw {
    fn below(&mut self, slots: u64) -> u64 {
        loop {
            // SplitMix64; reject the incomplete tail to sample slots uniformly.
            self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = self.0;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            value ^= value >> 31;
            if value < u64::MAX - u64::MAX % slots {
                return value % slots;
            }
        }
    }
}
