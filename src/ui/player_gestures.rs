use egui::{Pos2, Rect};

const TAP_WINDOW: f64 = 0.32;
const HOLD_DELAY: f64 = 0.42;
const DRAG_DISTANCE: f32 = 14.0;

#[derive(Clone, Copy)]
enum Mode {
    Pending,
    Vertical,
    Boost,
    Cancelled,
}

#[derive(Clone, Copy)]
struct Touch {
    start: Pos2,
    time: f64,
    right: bool,
    mode: Mode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Seek(bool),
    Level {
        right: bool,
        delta: f32,
        started: bool,
    },
    Speed(f32),
}

#[derive(Clone, Default)]
struct Gesture {
    touch: Option<Touch>,
    tap: Option<Touch>,
}

impl Gesture {
    fn begin(&mut self, pos: Pos2, time: f64, bounds: Rect) {
        self.touch = Some(Touch {
            start: pos,
            time,
            right: pos.x >= bounds.center().x,
            mode: Mode::Pending,
        });
    }

    fn update(&mut self, pos: Pos2, time: f64, height: f32, may_boost: bool) -> Option<Action> {
        let touch = self.touch.as_mut()?;
        let movement = pos - touch.start;
        let mut started = false;
        if matches!(touch.mode, Mode::Pending) && movement.length() > DRAG_DISTANCE {
            self.tap = None;
            touch.mode = if movement.y.abs() > movement.x.abs() * 1.2 {
                started = true;
                Mode::Vertical
            } else {
                Mode::Cancelled
            };
        }
        match touch.mode {
            Mode::Vertical => Some(Action::Level {
                right: touch.right,
                delta: -movement.y / (height * 0.62).max(1.0),
                started,
            }),
            Mode::Pending if touch.right && may_boost && time - touch.time >= HOLD_DELAY => {
                self.tap = None;
                touch.mode = Mode::Boost;
                Some(Action::Speed(1.5))
            }
            _ => None,
        }
    }

    fn finish(&mut self, time: f64, position: Option<Pos2>) -> Option<Action> {
        let touch = self.touch.take()?;
        if matches!(touch.mode, Mode::Boost) {
            return Some(Action::Speed(1.0));
        }
        let Some(pos) = position else {
            self.tap = None;
            return None;
        };
        if !matches!(touch.mode, Mode::Pending)
            || pos.distance(touch.start) > DRAG_DISTANCE
            || time - touch.time >= HOLD_DELAY
        {
            self.tap = None;
            return None;
        }
        if let Some(first) = self.tap.take() {
            if first.right == touch.right
                && time - first.time <= TAP_WINDOW
                && pos.distance(first.start) <= 56.0
            {
                return Some(Action::Seek(touch.right));
            }
        }
        self.tap = Some(Touch {
            start: pos,
            time,
            ..touch
        });
        None
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile {
    use super::*;
    use crate::app::App;
    use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, LayerId, Order};
    use std::time::Duration;

    #[cfg(target_os = "android")]
    use crate::platform::android::jni_bridge::player_brightness;
    #[cfg(target_os = "ios")]
    use crate::platform::ios::bridge::player_brightness;

    #[derive(Clone, Copy)]
    enum Feedback {
        Jump(i32),
        Level { right: bool, value: f32 },
        Speed,
    }

    #[derive(Clone, Default)]
    struct State {
        gesture: Gesture,
        base: f32,
        feedback: Option<(Feedback, f64)>,
    }

    pub(crate) fn frame(
        app: &mut App,
        ctx: &Context,
        bounds: Rect,
        live: bool,
        blocked: &[Rect],
        modal: bool,
    ) {
        let Some(item) = app.current_play.as_ref() else {
            return;
        };
        let Some(player) = app.player.as_ref() else {
            return;
        };
        let may_seek = !live || player.duration > 0.0;
        let may_boost = may_seek && !player.paused;
        let id = Id::new(("touch-playback-gestures", &item.progress_key));
        let mut state = ctx
            .data(|data| data.get_temp::<State>(id))
            .unwrap_or_default();
        let (time, pos, press, release, down, cancelled) = ctx.input(|input| {
            (
                input.time,
                input.pointer.interact_pos(),
                input.pointer.primary_pressed(),
                input.pointer.primary_released(),
                input.pointer.primary_down(),
                !input.focused
                    || input.events.iter().any(|event| {
                        matches!(
                            event,
                            egui::Event::Touch {
                                phase: egui::TouchPhase::Cancel,
                                ..
                            }
                        )
                    }),
            )
        });
        let action = if cancelled || modal {
            state.gesture.finish(time, None)
        } else {
            if press {
                if let Some(pos) = pos.filter(|&pos| {
                    bounds.contains(pos) && !blocked.iter().any(|rect| rect.contains(pos))
                }) {
                    state.gesture.begin(pos, time, bounds);
                } else {
                    state.gesture.tap = None;
                }
            }
            if release {
                state.gesture.finish(time, pos)
            } else if down {
                pos.and_then(|pos| state.gesture.update(pos, time, bounds.height(), may_boost))
            } else {
                state.gesture.finish(time, None)
            }
        };
        if let Some(action) = action {
            let feedback = match action {
                Action::Seek(right) if may_seek => {
                    let step = crate::release_info::SEEK_STEP_SECONDS as i32;
                    let skip = if right { step } else { -step };
                    player.mpv.seek_relative(f64::from(skip));
                    Some(Feedback::Jump(skip))
                }
                Action::Seek(_) => None,
                Action::Speed(speed) => {
                    player.mpv.set_speed(speed);
                    if speed == 1.0 {
                        state.feedback = None;
                        None
                    } else {
                        Some(Feedback::Speed)
                    }
                }
                Action::Level {
                    right,
                    delta,
                    started,
                } => {
                    if started {
                        state.base = if right {
                            app.volume as f32 / 100.0
                        } else {
                            player_brightness(-1.0).unwrap_or(0.5)
                        };
                    }
                    let value = (state.base + delta).clamp(0.0, 1.0);
                    if right {
                        let volume = (value * 100.0).round() as i64;
                        if volume != app.volume {
                            app.volume = volume;
                            player.mpv.set_volume(volume);
                        }
                    } else if !matches!(state.feedback, Some((Feedback::Level { value: old, .. }, _)) if (old - value).abs() < 0.003)
                    {
                        player_brightness(value);
                    }
                    Some(Feedback::Level { right, value })
                }
            };
            if let Some(feedback) = feedback {
                state.feedback = Some((feedback, time + 1.1));
            }
        }
        if let Some(touch) = state.gesture.touch {
            if touch.right && matches!(touch.mode, Mode::Pending) && may_boost {
                ctx.request_repaint_after(Duration::from_secs_f64(
                    (touch.time + HOLD_DELAY - time).max(0.0),
                ));
            }
        }
        if let Some((feedback, expiry)) = state.feedback {
            if time < expiry || matches!(feedback, Feedback::Speed) {
                draw(app, ctx, bounds, feedback);
                if !matches!(feedback, Feedback::Speed) {
                    ctx.request_repaint_after(Duration::from_secs_f64(expiry - time));
                }
            } else {
                state.feedback = None;
            }
        }
        ctx.data_mut(|data| data.insert_temp(id, state));
    }

    fn draw(app: &App, ctx: &Context, bounds: Rect, feedback: Feedback) {
        let language = app.settings.language;
        let (text, right, level) = match feedback {
            Feedback::Jump(seconds) => (format!("{seconds:+} s"), seconds > 0, None),
            Feedback::Speed => ("1,5×".into(), true, None),
            Feedback::Level { right, value } => {
                let label = if right {
                    language.text("Lautstärke", "Volume")
                } else {
                    language.text("Helligkeit", "Brightness")
                };
                (
                    format!("{label} {:.0} %", value * 100.0),
                    right,
                    Some(value),
                )
            }
        };
        let center = pos2(
            bounds.left() + bounds.width() * if right { 0.75 } else { 0.25 },
            bounds.center().y,
        );
        let card = Rect::from_center_size(
            center,
            vec2(
                (bounds.width() * 0.42).min(190.0),
                if level.is_some() { 70.0 } else { 48.0 },
            ),
        );
        let painter = ctx.layer_painter(LayerId::new(
            Order::Foreground,
            Id::new("touch-playback-feedback"),
        ));
        painter.rect_filled(card, 13.0, Color32::from_black_alpha(218));
        painter.text(
            center - vec2(0.0, if level.is_some() { 11.0 } else { 0.0 }),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(17.0),
            Color32::WHITE,
        );
        if let Some(value) = level {
            let track = Rect::from_min_size(
                card.left_bottom() + vec2(16.0, -23.0),
                vec2(card.width() - 32.0, 7.0),
            );
            painter.rect_filled(track, 4.0, Color32::from_white_alpha(85));
            painter.rect_filled(
                Rect::from_min_size(track.min, vec2(track.width() * value, track.height())),
                4.0,
                Color32::from_rgb(170, 218, 255),
            );
        }
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub(super) use mobile::frame;

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn taps_require_the_same_side_and_holding_restores_normal_speed() {
        let bounds = Rect::from_min_max(Pos2::ZERO, pos2(640.0, 360.0));
        let left = pos2(150.0, 140.0);
        let right = pos2(490.0, 140.0);
        let mut gesture = Gesture::default();
        gesture.begin(left, 0.0, bounds);
        assert_eq!(gesture.finish(0.05, Some(left)), None);
        gesture.begin(right, 0.1, bounds);
        assert_eq!(gesture.finish(0.15, Some(right)), None);
        gesture.begin(right, 0.2, bounds);
        assert_eq!(gesture.finish(0.25, Some(right)), Some(Action::Seek(true)));
        gesture.begin(right, 1.0, bounds);
        assert_eq!(
            gesture.update(right, 1.5, bounds.height(), true),
            Some(Action::Speed(1.5))
        );
        assert_eq!(gesture.finish(1.6, None), Some(Action::Speed(1.0)));
    }

    #[test]
    fn vertical_swipes_lock_the_initial_side_and_do_not_turn_into_holds_or_taps() {
        let bounds = Rect::from_min_max(Pos2::ZERO, pos2(640.0, 360.0));
        let start = pos2(321.0, 170.0);
        let end = pos2(312.0, 130.0);
        let mut gesture = Gesture::default();
        gesture.begin(start, 1.0, bounds);
        assert!(matches!(gesture.update(end, 1.2, bounds.height(), true),
            Some(Action::Level {right: true, delta, started: true}) if delta > 0.0));
        assert!(matches!(
            gesture.update(end, 1.8, bounds.height(), true),
            Some(Action::Level {
                right: true,
                started: false,
                ..
            })
        ));
        assert_eq!(gesture.finish(1.9, Some(end)), None);
    }
}
