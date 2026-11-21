use std::{
    hash::{BuildHasher, RandomState},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui;
use rust_modem_safety_island::{
    DispatchError, EventKind, RaceStep, ScenarioResult, T300_VALUES_MS, race_events_with_delays,
    run_all,
};

#[path = "rrc-gui/lifecycle.rs"]
mod lifecycle;
#[path = "rrc-gui/state_diagram.rs"]
mod state_diagram;
use lifecycle::lifecycle_view;
use state_diagram::{Edge, highlighted_path};

const REPLAY_STEP: Duration = Duration::from_millis(400);
const INK: egui::Color32 = egui::Color32::from_rgb(13, 22, 25);
const PANEL: egui::Color32 = egui::Color32::from_rgb(20, 34, 38);
const RULE: egui::Color32 = egui::Color32::from_rgb(45, 69, 73);
const TEXT: egui::Color32 = egui::Color32::from_rgb(226, 239, 237);
const MUTED: egui::Color32 = egui::Color32::from_rgb(154, 179, 179);
const SIGNAL: egui::Color32 = egui::Color32::from_rgb(109, 233, 204);
const REJECTED: egui::Color32 = egui::Color32::from_rgb(255, 194, 111);

fn kicker(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .monospace()
        .size(11.0)
        .color(MUTED)
}

fn instrument_panel() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0_f32, RULE))
        .corner_radius(12)
        .inner_margin(16)
}

struct SetupDelayGenerator(u64);

impl Default for SetupDelayGenerator {
    fn default() -> Self {
        Self(RandomState::new().hash_one(0_u8))
    }
}

impl SetupDelayGenerator {
    fn sample_ms(&mut self, t300_ms: u64) -> u64 {
        const SLOTS: u64 = 101;
        loop {
            // SplitMix64; reject the short tail to keep the millisecond slots uniform.
            self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = self.0;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            value ^= value >> 31;
            if value < u64::MAX - u64::MAX % SLOTS {
                return t300_ms - 50 + value % SLOTS;
            }
        }
    }
}

#[derive(Default)]
struct App {
    pending: Option<mpsc::Receiver<[RaceStep; 2]>>,
    result: Option<Result<[RaceStep; 2], &'static str>>,
    replay_started: Option<Instant>,
    setup_generator: SetupDelayGenerator,
    t300_index: usize,
    sampled_setup_ms: Option<u64>,
    scenarios: Vec<ScenarioResult>,
    procedure: Procedure,
    scenario_started: Option<Instant>,
    seed: String,
    stochastic_t300_index: usize,
    stochastic: Option<(u64, u64, rust_modem_safety_island::StochasticRun)>,
    stochastic_started: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Procedure {
    #[default]
    Race,
    Stochastic,
    Scenario(usize),
}

impl App {
    fn start(&mut self, ctx: &egui::Context) {
        let t300_ms = T300_VALUES_MS[self.t300_index];
        let setup_ms = self.setup_generator.sample_ms(t300_ms);
        self.sampled_setup_ms = Some(setup_ms);
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = send.send(race_events_with_delays(
                Duration::from_millis(t300_ms),
                Duration::from_millis(setup_ms),
            ));
            ctx.request_repaint();
        });
        self.pending = Some(receive);
        self.result = None;
        self.replay_started = None;
    }

    fn stochastic_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Stochastic Run");
        ui.horizontal(|ui| {
            ui.label(kicker("Seed"));
            ui.add(egui::TextEdit::singleline(&mut self.seed).desired_width(155.0));
            egui::ComboBox::from_id_salt("stochastic-t300")
                .selected_text(format!(
                    "T300 · {} ms",
                    T300_VALUES_MS[self.stochastic_t300_index]
                ))
                .width(120.0)
                .show_ui(ui, |ui| {
                    for (index, ms) in T300_VALUES_MS.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.stochastic_t300_index,
                            index,
                            format!("{ms} ms"),
                        );
                    }
                });
            let seed = self.seed.trim().parse::<u64>();
            let button = egui::Button::new(egui::RichText::new("執行 / Run").strong().color(INK))
                .fill(SIGNAL);
            if ui.add_enabled(seed.is_ok(), button).clicked()
                && let Ok(seed) = seed
            {
                let t300_ms = T300_VALUES_MS[self.stochastic_t300_index];
                self.stochastic = Some((
                    seed,
                    t300_ms,
                    rust_modem_safety_island::stochastic_run(seed, t300_ms),
                ));
                self.stochastic_started = Some(Instant::now());
            }
            if seed.is_err() {
                ui.label(kicker("Enter an unsigned 64-bit seed").color(REJECTED));
            }
        });
        if let Some((seed, t300_ms, run)) = &self.stochastic {
            ui.label(kicker(&format!(
                "Seed {seed} · T300 {t300_ms} ms · simulated time"
            )));
            lifecycle::lifecycle_trace(
                ui,
                &run.scenario,
                self.stochastic_started,
                Some(&run.steps_ms),
            );
        } else {
            ui.label(kicker(
                "Run a seeded trajectory from Idle. Timings are simulated.",
            ));
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(receiver) = &self.pending {
            self.result = match receiver.try_recv() {
                Ok(trace) => {
                    self.replay_started = Some(Instant::now());
                    Some(Ok(trace))
                }
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("Race worker stopped")),
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if self.result.is_some() {
                self.pending = None;
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(INK).inner_margin(22))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(kicker("PROCEDURE"));
                    let previous = self.procedure;
                    egui::ComboBox::from_id_salt("procedure")
                        .selected_text(match previous {
                            Procedure::Race => "T300 event race",
                            Procedure::Stochastic => "Stochastic Run",
                            Procedure::Scenario(i) => self.scenarios[i].name,
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.procedure,
                                Procedure::Race,
                                "T300 event race",
                            );
                            ui.selectable_value(
                                &mut self.procedure,
                                Procedure::Stochastic,
                                "Stochastic Run",
                            );
                            for (i, scenario) in self.scenarios.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.procedure,
                                    Procedure::Scenario(i),
                                    scenario.name,
                                );
                            }
                        });
                    if previous != self.procedure {
                        self.scenario_started = None;
                    }
                });
                ui.add_space(8.0);
                match self.procedure {
                    Procedure::Scenario(index) => {
                        lifecycle_view(ui, &self.scenarios[index], &mut self.scenario_started);
                        return;
                    }
                    Procedure::Stochastic => {
                        self.stochastic_view(ui);
                        return;
                    }
                    Procedure::Race => {}
                }
                ui.horizontal(|ui| {
                    ui.label(kicker("RADIO CONTROL  /  LAB 01").color(SIGNAL));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled_ui(self.pending.is_none(), |ui| {
                            let previous = self.t300_index;
                            egui::ComboBox::from_id_salt("t300")
                                .selected_text(format!("T300  ·  {} ms", T300_VALUES_MS[previous]))
                                .width(130.0)
                                .show_ui(ui, |ui| {
                                    for (index, ms) in T300_VALUES_MS.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self.t300_index,
                                            index,
                                            format!("{ms} ms"),
                                        );
                                    }
                                });
                            if previous != self.t300_index {
                                self.sampled_setup_ms = None;
                                self.result = None;
                                self.replay_started = None;
                            }
                        });
                    });
                });
                ui.add_space(7.0);
                ui.label(
                    egui::RichText::new("RRC Event Race")
                        .size(32.0)
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(3.0);
                let running = self.pending.is_some();
                ui.horizontal(|ui| {
                    let setup = self.sampled_setup_ms.map_or_else(
                        || "RRCSetup arrival sampled at T300 ± 50 ms".to_owned(),
                        |ms| format!("RRCSetup  ·  {ms} ms (sampled)"),
                    );
                    ui.label(egui::RichText::new(setup).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = egui::RichText::new("執行 / Run").strong().color(INK);
                        let button = egui::Button::new(label)
                            .fill(if running { MUTED } else { SIGNAL })
                            .corner_radius(6)
                            .min_size(egui::vec2(110.0, 34.0));
                        if ui.add_enabled(!running, button).clicked() {
                            self.start(ctx);
                        }
                    });
                });
                if self.pending.is_some() {
                    // The connecting state in the diagram also shows progress.
                    // Also detect a worker that exits before requesting repaint.
                    ctx.request_repaint_after(Duration::from_millis(50));
                }
                ui.add_space(12.0);
                let (path, shown) = match (&self.result, self.replay_started) {
                    (Some(Ok(trace)), Some(started)) => {
                        let elapsed = started.elapsed();
                        let count =
                            1 + (elapsed.as_millis() / REPLAY_STEP.as_millis()).min(2) as usize;
                        if count < 3 {
                            ctx.request_repaint_after(
                                (REPLAY_STEP * count as u32).saturating_sub(elapsed),
                            );
                        }
                        (highlighted_path(trace), count)
                    }
                    _ => ([Edge::Start; 3], usize::from(self.pending.is_some())),
                };
                instrument_panel().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(kicker("01  /  STATE MAP").color(TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(kicker("● Rejected").color(REJECTED));
                            ui.label(kicker("● Transition").color(SIGNAL));
                        });
                    });
                    ui.add_space(4.0);
                    state_diagram::show(ui, &path[..shown]);
                });
                ui.add_space(12.0);
                instrument_panel().show(ui, |ui| {
                    ui.set_min_size(egui::vec2(ui.available_width(), 112.0));
                    ui.label(kicker("02  /  RECEIVE ORDER").color(TEXT));
                    ui.add_space(8.0);
                    match &self.result {
                        Some(Ok(trace)) => {
                            ui.label(kicker("RRCSetupRequest: Idle → Connecting"));
                            for (index, step) in
                                trace.iter().take(shown.saturating_sub(1)).enumerate()
                            {
                                ui.add_space(5.0);
                                ui.separator();
                                ui.add_space(4.0);
                                event_row(ui, index, step);
                            }
                        }
                        Some(Err(message)) => {
                            ui.label(egui::RichText::new(*message).color(REJECTED));
                        }
                        None => {
                            ui.label(
                                egui::RichText::new(if self.pending.is_some() {
                                    "Waiting for both events…"
                                } else {
                                    "Run one race to see the actual receive order."
                                })
                                .color(MUTED),
                            );
                        }
                    }
                });
            });
    }
}

fn event_row(ui: &mut egui::Ui, index: usize, step: &RaceStep) {
    let event = match step.event {
        EventKind::RrcSetup => "RRCSetup",
        EventKind::T300Expiry => "T300Expiry",
        other => unreachable!("unexpected displayed race event: {other:?}"),
    };
    let (color, detail) = match step.result {
        Ok(outcome) => (
            SIGNAL,
            format!(
                "{outcome:?} · CM-{:?} → CM-{:?}",
                step.before.cm_state(),
                step.after.cm_state()
            ),
        ),
        Err(DispatchError::UnexpectedEvent { state, .. }) => (
            REJECTED,
            format!(
                "UnexpectedEvent: {event} in {state:?} · CM-{:?} (unchanged)",
                step.before.cm_state()
            ),
        ),
    };
    ui.horizontal(|ui| {
        ui.label(kicker(if index == 0 { "01" } else { "02" }).color(color));
        ui.label(egui::RichText::new(event).strong().color(TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(kicker(&format!("{:?} → {:?}", step.before, step.after)));
        });
    });
    ui.label(kicker(&detail).color(color));
}

/// Selects the GUI's initial procedure from one optional CLI argument.
///
/// No argument boots into the T300 race. `stochastic` replays the default
/// seeded run; a scenario name boots into that scenario with replay started.
fn resolve_procedure(args: &[String], scenarios: &[ScenarioResult]) -> Result<Procedure, String> {
    let name = match args {
        [] => return Ok(Procedure::Race),
        [name] => name,
        _ => return Err(usage(scenarios)),
    };
    match name.as_str() {
        "race" => Ok(Procedure::Race),
        "stochastic" => Ok(Procedure::Stochastic),
        name => scenarios
            .iter()
            .position(|scenario| scenario.name == name)
            .map(Procedure::Scenario)
            .ok_or_else(|| usage(scenarios)),
    }
}

fn usage(scenarios: &[ScenarioResult]) -> String {
    let mut names: Vec<&str> = scenarios.iter().map(|scenario| scenario.name).collect();
    names.sort_unstable();
    format!("usage: rrc-gui [race|stochastic|{}]", names.join("|"))
}

fn main() -> eframe::Result {
    let scenarios = run_all();
    let procedure =
        match resolve_procedure(&std::env::args().skip(1).collect::<Vec<_>>(), &scenarios) {
            Ok(procedure) => procedure,
            Err(usage) => {
                eprintln!("{usage}");
                std::process::exit(2);
            }
        };
    let seed = 42_u64;
    let t300_ms = T300_VALUES_MS[0];
    eframe::run_native(
        "RRC Event Race",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 680.0])
                .with_min_inner_size([640.0, 670.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(TEXT);
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, RULE);
            cc.egui_ctx.set_visuals(visuals);
            #[cfg(target_os = "macos")]
            if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Hiragino Sans GB.ttc") {
                let mut fonts = egui::FontDefinitions::default();
                fonts
                    .font_data
                    .insert("cjk".into(), egui::FontData::from_owned(bytes).into());
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("cjk".into());
                cc.egui_ctx.set_fonts(fonts);
            }
            Ok(Box::new(App {
                scenarios,
                seed: seed.to_string(),
                procedure,
                scenario_started: matches!(procedure, Procedure::Scenario(_)).then(Instant::now),
                stochastic: matches!(procedure, Procedure::Stochastic).then(|| {
                    (
                        seed,
                        t300_ms,
                        rust_modem_safety_island::stochastic_run(seed, t300_ms),
                    )
                }),
                stochastic_started: matches!(procedure, Procedure::Stochastic).then(Instant::now),
                ..App::default()
            }))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_modem_safety_island::{StateKind, TransitionOutcome};

    #[test]
    fn setup_delay_is_seeded_and_varies_around_each_spec_timer() {
        assert_eq!(T300_VALUES_MS, [100, 200, 300, 400, 600, 1000, 1500, 2000]);
        for t300 in T300_VALUES_MS {
            let mut first = SetupDelayGenerator(42);
            let mut second = SetupDelayGenerator(42);
            let delays: [u64; 64] = std::array::from_fn(|_| {
                let delay = first.sample_ms(t300);
                assert_eq!(delay, second.sample_ms(t300));
                delay
            });
            assert!(delays.iter().all(|&ms| ms.abs_diff(t300) <= 50));
            assert!(delays.iter().any(|&ms| ms < t300));
            assert!(delays.iter().any(|&ms| ms > t300));
        }
    }

    #[test]
    fn highlights_only_the_observed_branch_and_rejection_in_order() {
        for (winner, final_state, outcome, loser, accepted, rejected) in [
            (
                EventKind::RrcSetup,
                StateKind::Connected,
                TransitionOutcome::ConnectionEstablished,
                EventKind::T300Expiry,
                Edge::Setup,
                Edge::LateExpiry,
            ),
            (
                EventKind::T300Expiry,
                StateKind::Idle,
                TransitionOutcome::ConnectionFailed,
                EventKind::RrcSetup,
                Edge::Expiry,
                Edge::LateSetup,
            ),
        ] {
            let trace = [
                RaceStep {
                    event: winner,
                    before: StateKind::Connecting,
                    after: final_state,
                    result: Ok(outcome),
                },
                RaceStep {
                    event: loser,
                    before: final_state,
                    after: final_state,
                    result: Err(DispatchError::UnexpectedEvent {
                        state: final_state,
                        event: loser,
                    }),
                },
            ];
            assert_eq!(highlighted_path(&trace), [Edge::Start, accepted, rejected]);
        }
    }

    #[test]
    fn cli_arg_selects_initial_procedure() {
        let scenarios = run_all();
        assert_eq!(resolve_procedure(&[], &scenarios), Ok(Procedure::Race));
        assert_eq!(
            resolve_procedure(&["race".to_owned()], &scenarios),
            Ok(Procedure::Race)
        );
        assert_eq!(
            resolve_procedure(&["stochastic".to_owned()], &scenarios),
            Ok(Procedure::Stochastic)
        );
        let index = scenarios
            .iter()
            .position(|scenario| scenario.name == "suspend-and-resume")
            .expect("scenario exists");
        assert_eq!(
            resolve_procedure(&["suspend-and-resume".to_owned()], &scenarios),
            Ok(Procedure::Scenario(index))
        );
        assert!(resolve_procedure(&["no-such-scenario".to_owned()], &scenarios).is_err());
        assert!(resolve_procedure(&["race".to_owned(), "extra".to_owned()], &scenarios).is_err());
    }
}
