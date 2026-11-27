use std::{cell::RefCell, time::Instant};

use gpui::{App, EntityId, ListState, Window, px};

const STEP: f64 = 1. / 240.;
const TENSION: f64 = 200.;
const FRICTION: f64 = 35.;
const MASS: f64 = 2.;
const THRESHOLD: f64 = 0.001;
const MAX_LAG: f64 = 2.;
pub const PAGE_FRACTION: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub from: f64,
    pub target: f64,
    vel: f64,
    acc: f64,
    last: Option<Instant>,
}

impl Spring {
    pub fn new(from: f64, target: f64) -> Self {
        Self { from, target, vel: 0., acc: 0., last: None }
    }

    pub fn retarget(&mut self, from: f64, target: f64) {
        self.from = from;
        self.target = target;
    }

    pub fn shift(&mut self, by: f64) {
        self.from += by;
        self.target += by;
    }

    fn integrate(&self, vel: f64, from: f64) -> (f64, f64, f64) {
        let accel = (-TENSION * (from - self.target) - FRICTION * vel) / MASS;
        let vel = vel + accel * STEP;
        (from + vel * STEP, vel, accel)
    }

    pub fn advance(&mut self, now: Instant) -> (f64, bool) {
        let Some(last) = self.last.replace(now) else {
            return (self.from, false);
        };
        self.acc = (self.acc + now.saturating_duration_since(last).as_secs_f64()).min(MAX_LAG);
        while self.acc > STEP {
            self.acc -= STEP;
            let (next, vel, accel) = self.integrate(self.vel, self.from);
            self.vel = vel;
            let crossed = next == self.target
                || (next < self.target && self.from > self.target)
                || (next > self.target && self.from < self.target);
            if crossed || (accel * STEP).abs() < THRESHOLD {
                self.from = self.target;
                return (self.target, true);
            }
            self.from = next;
        }
        let mut shown = self.from;
        if self.acc > 0. {
            let (next, ..) = self.integrate(self.vel, self.from);
            shown += (next - self.from) * (self.acc / STEP);
        }
        (shown, false)
    }
}

struct Run {
    list: ListState,
    view: EntityId,
    spring: Spring,
    shown: f64,
}

thread_local! {
    static RUN: RefCell<Option<Run>> = const { RefCell::new(None) };
}

fn top_of(list: &ListState) -> f64 {
    -f64::from(f32::from(list.scroll_px_offset_for_scrollbar().y))
}

fn max_of(list: &ListState) -> f64 {
    f64::from(f32::from(list.max_offset_for_scrollbar().y))
}

pub fn page(list: &ListState, up: bool, view: EntityId, window: &mut Window, reduced: bool) {
    let h = f32::from(list.viewport_bounds().size.height);
    let d = f64::from(if h > 0. { h * PAGE_FRACTION } else { 400. });
    let top = top_of(list);
    scroll_to(list, if up { top - d } else { top + d }, view, window, reduced);
}

pub fn to_top(list: &ListState, view: EntityId, window: &mut Window, reduced: bool) {
    scroll_to(list, 0., view, window, reduced);
}

pub fn scroll_to(list: &ListState, to: f64, view: EntityId, window: &mut Window, reduced: bool) {
    let top = top_of(list);
    let target = to.clamp(0., max_of(list).max(0.));
    if reduced {
        stop();
        let list = list.clone();
        window.on_next_frame(move |window, cx| list.scroll_by_notify(px((target - top) as f32), view, window, cx));
        return;
    }
    let start = RUN.with(|r| {
        let mut r = r.borrow_mut();
        match r.as_mut().filter(|run| run.view == view) {
            Some(run) => {
                run.spring.retarget(top, target);
                run.shown = top;
                false
            }
            None => {
                let idle = r.is_none();
                *r = Some(Run { list: list.clone(), view, spring: Spring::new(top, target), shown: top });
                idle
            }
        }
    });
    if start {
        window.on_next_frame(step);
    }
}

pub fn stop() {
    RUN.with(|r| r.borrow_mut().take());
}

fn step(window: &mut Window, cx: &mut App) {
    let now = Instant::now();
    let Some((list, view, delta, done)) = RUN.with(|r| {
        let mut r = r.borrow_mut();
        let run = r.as_mut()?;
        let actual = top_of(&run.list);
        run.spring.shift(actual - run.shown);
        run.shown = actual;
        let (pos, done) = run.spring.advance(now);
        let delta = pos - run.shown;
        run.shown = pos;
        let out = (run.list.clone(), run.view, delta, done);
        if done {
            *r = None;
        }
        Some(out)
    }) else {
        return;
    };
    list.scroll_by_notify(px(delta as f32), view, window, cx);
    if !done {
        window.on_next_frame(step);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn run(mut s: Spring, frame_ms: u64) -> Vec<(u64, f64)> {
        let t0 = Instant::now();
        let mut out = vec![(0, s.advance(t0).0)];
        for i in 1..400 {
            let (pos, done) = s.advance(t0 + Duration::from_millis(i * frame_ms));
            out.push((i * frame_ms, pos));
            if done {
                break;
            }
        }
        out
    }

    #[test]
    fn a_page_settles_on_its_target_without_overshoot_like_the_client_spring() {
        let frames = run(Spring::new(1000., 460.), 16);
        let (end_ms, end) = *frames.last().unwrap();
        assert_eq!(end, 460.);
        assert!((350..=900).contains(&end_ms), "{end_ms}");
        assert!(frames.windows(2).all(|w| w[1].1 <= w[0].1), "{frames:?}");
        assert!(frames.iter().all(|(_, p)| *p >= 460.));
        let half = frames.iter().find(|(_, p)| *p <= 730.).unwrap().0;
        assert!((80..=260).contains(&half), "{half}");
    }

    #[test]
    fn the_first_frame_only_starts_the_clock() {
        let mut s = Spring::new(0., 100.);
        let t0 = Instant::now();
        assert_eq!(s.advance(t0), (0., false));
        let (p, done) = s.advance(t0 + Duration::from_millis(16));
        assert!(!done && p > 0. && p < 10., "{p}");
    }

    #[test]
    fn a_second_page_keeps_the_velocity_and_moves_the_target() {
        let mut s = Spring::new(0., 540.);
        let t0 = Instant::now();
        s.advance(t0);
        let (p, _) = s.advance(t0 + Duration::from_millis(100));
        s.retarget(p, p + 540.);
        let (q, _) = s.advance(t0 + Duration::from_millis(116));
        let fresh = {
            let mut f = Spring::new(p, p + 540.);
            f.advance(t0 + Duration::from_millis(100));
            f.advance(t0 + Duration::from_millis(116)).0
        };
        assert!(q - p > fresh - p, "{q} {fresh} {p}");
    }

    #[test]
    fn content_moving_underneath_shifts_the_whole_run() {
        let mut s = Spring::new(100., 200.);
        s.shift(50.);
        assert_eq!((s.from, s.target), (150., 250.));
    }
}
