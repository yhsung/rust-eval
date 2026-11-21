use std::time::Instant;

use eframe::egui;
use rust_modem_safety_island::ScenarioResult;

use super::{INK, MUTED, REJECTED, REPLAY_STEP, SIGNAL, TEXT, instrument_panel, kicker};

pub(super) fn lifecycle_view(
    ui: &mut egui::Ui,
    scenario: &ScenarioResult,
    started: &mut Option<Instant>,
) {
    ui.horizontal(|ui| {
        ui.heading(scenario.name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button =
                egui::Button::new(egui::RichText::new("回放 / Replay").strong().color(INK))
                    .fill(SIGNAL)
                    .min_size(egui::vec2(110., 34.));
            if ui.add(button).clicked() {
                *started = Some(Instant::now());
            }
        });
    });
    lifecycle_trace(ui, scenario, *started, None);
}

pub(super) fn lifecycle_trace(
    ui: &mut egui::Ui,
    scenario: &ScenarioResult,
    started: Option<Instant>,
    steps_ms: Option<&[u64]>,
) {
    ui.add_space(12.0);
    let shown = started.map_or(0, |start| {
        let elapsed = start.elapsed();
        let count = (elapsed.as_millis() / REPLAY_STEP.as_millis()) as usize;
        if count < scenario.steps.len() {
            ui.ctx()
                .request_repaint_after((REPLAY_STEP * (count as u32 + 1)).saturating_sub(elapsed));
        }
        count
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        instrument_panel().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(kicker("01  /  STATE PATH").color(TEXT));
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    state_chip(ui, "Idle", SIGNAL);
                    for step in scenario.steps.iter().take(shown) {
                        let (state, color) = match &step.result {
                            Ok(transition) if transition.state.kind() != step.before => {
                                (transition.state.kind(), SIGNAL)
                            }
                            Ok(_) => continue,
                            Err(_) => (step.before, REJECTED),
                        };
                        ui.label(kicker("→").color(color));
                        state_chip(ui, &format!("{state:?}"), color);
                    }
                });
            });
        });
        ui.add_space(12.0);
        instrument_panel().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(kicker("02  /  EVENT TRACE").color(TEXT));
            if started.is_none() {
                ui.label(kicker(
                    "Replay to inspect the state path and dispatch results.",
                ));
            }
            for (index, step) in scenario.steps.iter().enumerate() {
                ui.separator();
                let revealed = index < shown;
                let result = revealed.then(|| match &step.result {
                    Ok(transition) => (
                        transition.state.kind(),
                        SIGNAL,
                        format!(
                            "{:?} · CM-{:?} → CM-{:?}",
                            transition.outcome,
                            step.cm_before,
                            transition.state.cm_state()
                        ),
                    ),
                    Err(error) => (
                        step.before,
                        REJECTED,
                        format!("{error:?} · CM-{:?} (unchanged)", step.cm_before),
                    ),
                });
                ui.horizontal(|ui| {
                    if let Some(times) = steps_ms {
                        ui.label(kicker(&format!("{:>5} ms", times[index])));
                    } else {
                        ui.label(kicker(&format!("{:02}", index + 1)));
                    }
                    ui.label(egui::RichText::new(step.event).strong().color(if revealed {
                        TEXT
                    } else {
                        MUTED
                    }));
                    if let Some((after, color, _)) = &result {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                kicker(&format!("{:?} → {after:?}", step.before)).color(*color),
                            );
                        });
                    }
                });
                if let Some((_, color, detail)) = result {
                    ui.label(kicker(&detail).color(color));
                }
            }
        });
    });
}

fn state_chip(ui: &mut egui::Ui, state: &str, color: egui::Color32) {
    egui::Frame::new()
        .fill(INK)
        .stroke(egui::Stroke::new(1.0_f32, color))
        .corner_radius(5)
        .inner_margin(6)
        .show(ui, |ui| {
            ui.label(kicker(state).color(color));
        });
}
