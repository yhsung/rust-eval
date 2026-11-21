use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui;
use rust_modem_safety_island::{DispatchError, RaceStep, race_events};

#[path = "rrc-gui/state_diagram.rs"]
mod state_diagram;
use state_diagram::{Edge, highlighted_path};

const REPLAY_STEP: Duration = Duration::from_millis(400);

#[derive(Default)]
struct App {
    pending: Option<mpsc::Receiver<[RaceStep; 2]>>,
    result: Option<Result<[RaceStep; 2], &'static str>>,
    replay_started: Option<Instant>,
}

impl App {
    fn start(&mut self, ctx: &egui::Context) {
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = send.send(race_events());
            ctx.request_repaint();
        });
        self.pending = Some(receive);
        self.result = None;
        self.replay_started = None;
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RRC Event Race");
            ui.label("SetupComplete vs. T300Expiry · shared 10 ms demo deadline");
            ui.add_space(12.0);
            let running = self.pending.is_some();
            if ui
                .add_enabled(!running, egui::Button::new("執行 / Run"))
                .clicked()
            {
                self.start(ctx);
            }
            if self.pending.is_some() {
                ui.spinner();
                // Also detect a worker that exits before requesting repaint.
                ctx.request_repaint_after(Duration::from_millis(50));
            }
            ui.add_space(12.0);
            let (path, shown) = match (&self.result, self.replay_started) {
                (Some(Ok(trace)), Some(started)) => {
                    let elapsed = started.elapsed();
                    let count = 1 + (elapsed.as_millis() / REPLAY_STEP.as_millis()).min(2) as usize;
                    if count < 3 {
                        ctx.request_repaint_after(
                            (REPLAY_STEP * count as u32).saturating_sub(elapsed),
                        );
                    }
                    (highlighted_path(trace), count)
                }
                _ => ([Edge::Start; 3], usize::from(self.pending.is_some())),
            };
            state_diagram::show(ui, &path[..shown]);
            ui.add_space(12.0);
            match &self.result {
                Some(Ok(trace)) => {
                    ui.label("StartConnection: Idle → Connecting");
                    for (index, step) in trace.iter().take(shown.saturating_sub(1)).enumerate() {
                        ui.add_space(8.0);
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("{}. {:?}", index + 1, step.event))
                                    .strong(),
                            );
                            ui.monospace(format!("{:?} → {:?}", step.before, step.after));
                            match step.result {
                                Ok(outcome) => ui.monospace(format!("{outcome:?}")),
                                Err(DispatchError::UnexpectedEvent { state, event }) => {
                                    ui.monospace(format!("UnexpectedEvent: {event:?} in {state:?}"))
                                }
                            };
                        });
                    }
                }
                Some(Err(message)) => {
                    ui.label(*message);
                }
                None if !running => {
                    ui.label("Run one race to see the actual receive order.");
                }
                None => {}
            }
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "RRC Event Race",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([740.0, 570.0])
                .with_min_inner_size([620.0, 570.0]),
            ..Default::default()
        },
        Box::new(|cc| {
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
            Ok(Box::new(App::default()))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_modem_safety_island::{EventKind, StateKind, TransitionOutcome};

    #[test]
    fn highlights_only_the_observed_branch_and_rejection_in_order() {
        for (winner, final_state, outcome, loser, accepted, rejected) in [
            (
                EventKind::SetupComplete,
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
                EventKind::SetupComplete,
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
}
