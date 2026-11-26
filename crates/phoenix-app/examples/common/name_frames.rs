use std::time::{Duration, Instant};

use gpui::{AnyWindowHandle, HeadlessAppContext, px};

pub fn env_point(key: &str) -> Option<(f32, f32)> {
    let v = std::env::var(key).ok()?;
    let (x, y) = v.split_once(',')?;
    Some((x.trim().parse::<f32>().ok()?, y.trim().parse::<f32>().ok()?))
}

pub fn move_mouse(cx: &mut HeadlessAppContext, win: AnyWindowHandle, (x, y): (f32, f32)) -> anyhow::Result<()> {
    let ev = gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
        position: gpui::point(px(x), px(y)),
        pressed_button: None,
        modifiers: gpui::Modifiers::default(),
    });
    cx.update_window(win, |_, window, cx| {
        window.dispatch_event(ev, cx);
    })?;
    Ok(())
}

fn pump(cx: &mut HeadlessAppContext, ms: u64) {
    let until = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < until {
        std::thread::sleep(Duration::from_millis(25));
        cx.run_until_parked();
    }
}

pub fn prepare(cx: &mut HeadlessAppContext, win: AnyWindowHandle, scale: f32) -> anyhow::Result<()> {
    cx.update_window(win, |_, window, _| window.activate_window())?;
    pump(cx, 300);
    cx.update_window(win, |_, window, _| window.set_scale_factor(scale))?;
    pump(cx, 200);
    if let Some(p) = env_point("PHX_MOUSE") {
        move_mouse(cx, win, p)?;
        pump(cx, 200);
    }
    cx.update_window(win, |_, window, _| window.refresh())?;
    pump(cx, 300);
    Ok(())
}

pub fn gradient_frames(cx: &mut HeadlessAppContext, win: AnyWindowHandle, spec: &str, out: &str, scale: f32) -> anyhow::Result<()> {
    prepare(cx, win, scale)?;
    for ms in spec.split(',').filter_map(|v| v.trim().parse::<u64>().ok()) {
        let img = cx.update_window(win, |_, window, _| {
            window.set_gradient_elapsed(Duration::from_millis(ms));
            window.render_to_image()
        })??;
        let path = format!("{}-g{ms:04}.png", out.trim_end_matches(".png"));
        img.save(&path)?;
        println!("wrote {path} ({}x{})", img.width(), img.height());
    }
    Ok(())
}

pub fn present_cpu(cx: &mut HeadlessAppContext, win: AnyWindowHandle, secs: u64, scale: f32) -> anyhow::Result<()> {
    let cpu = || -> f64 {
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
        let f: Vec<&str> = stat.rsplit_once(')').map_or("", |(_, r)| r).split_whitespace().collect();
        let ticks = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.);
        (ticks(11) + ticks(12)) / 100.
    };
    prepare(cx, win, scale)?;
    for (label, redraw, hovered) in [("animated, present only", false, true), ("animated, full redraw", true, true), ("idle", false, false)] {
        if !hovered {
            move_mouse(cx, win, (1., 1.))?;
            pump(cx, 300);
        }
        cx.update_window(win, |_, window, _| window.refresh())?;
        cx.simulate_frame_request(win);
        let (c0, t0) = (cpu(), Instant::now());
        let mut frames = 0u32;
        while t0.elapsed() < Duration::from_secs(secs) {
            std::thread::sleep(Duration::from_millis(16));
            if redraw {
                cx.update_window(win, |_, window, _| window.refresh())?;
            }
            cx.simulate_frame_request(win);
            frames += 1;
        }
        let animating = cx.update_window(win, |_, window, _| window.scrolls_gradients())?;
        println!(
            "present cpu {label}: {:.1} % of one core over {frames} frame requests, animating={animating}",
            (cpu() - c0) / t0.elapsed().as_secs_f64() * 100.
        );
    }
    Ok(())
}
