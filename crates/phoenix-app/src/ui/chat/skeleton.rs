use gpui::{AnyElement, Div, IntoElement, ParentElement, Styled, div, prelude::*, px};

use crate::theme::{font_scale, fz, generated::idx, z};

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^ (x >> 31)
    }

    pub fn int(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next() % u64::from(hi - lo + 1)) as u32
    }

    pub fn float(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() >> 40) as f32 / (1u64 << 24) as f32 * (hi - lo)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attachment {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Specs {
    pub groups: Vec<u32>,
    pub attachments: Vec<(usize, Attachment)>,
    pub total_height: f32,
    pub group_spacing: f32,
}

pub fn page_messages(window_height: f32, compact: bool, scale: f32) -> u32 {
    let h = (window_height - 145.).max(400.);
    let per = if compact { 65. } else { 91. } * scale;
    ((1.05 * (h / per).ceil()).ceil() as u32).clamp(10, 30)
}

pub fn generate(seed: u64, window_height: f32, compact: bool, scale: f32, group_spacing: f32) -> Specs {
    let k = page_messages(window_height, compact, scale);
    let groups_n = if compact { k } else { (0.87 * k as f32).round() as u32 };
    let attachments_n = ((groups_n as f32 / 30. * 8.).round() as u32).max(1);
    let mut rng = Rng::new(seed);
    let first = if compact { 26. } else { 44. };
    let rest = if compact { 26. } else { 22. };
    let mut total = 0.;
    let groups: Vec<u32> = (0..groups_n)
        .map(|_| {
            let n = rng.int(1, 4);
            total += group_spacing * scale + first * scale + (n - 1) as f32 * rest * scale;
            n
        })
        .collect();
    let mut pool: Vec<usize> = (0..groups.len()).collect();
    let mut attachments = Vec::new();
    while attachments.len() < attachments_n as usize && !pool.is_empty() {
        let a = Attachment { width: rng.int(140, 400) as f32, height: rng.int(100, 320) as f32 };
        let at = rng.int(0, pool.len() as u32 - 1) as usize;
        attachments.push((pool.remove(at), a));
        total += a.height + 6. * scale;
    }
    Specs { groups, attachments, total_height: total, group_spacing }
}

fn fill(opacity: f32) -> gpui::Hsla {
    let c = super::tk(idx::TEXT_DEFAULT);
    gpui::Hsla { a: c.a * opacity, ..c }
}

fn blob(width: f32, opacity: f32) -> Div {
    div().flex_none().w(fz(width)).h(fz(16.)).mt(fz(3.)).rounded(fz(8.)).bg(fill(opacity))
}

fn message(rng: &mut Rng, compact: bool, group_start: bool, name_w: f32, name_a: f32, attachment: Option<Attachment>, top: f32) -> AnyElement {
    let scale = font_scale();
    let line = || div().h(fz(22.)).flex().items_center().overflow_hidden();
    let content = blob(rng.int(120, 320) as f32, 0.06);
    let mut wrap = div().relative().w_full().flex_none().overflow_hidden().pr(fz(16.)).mt(z(top));
    if compact {
        let stamp_w = 50.;
        let stamp_a = rng.float(0.02, 0.08);
        let stamp = blob(stamp_w, stamp_a).ml(fz(-64.)).mr(fz(4.)).when(!group_start, |d| d.invisible());
        let name = div().mr(fz(8.)).flex().items_center().child(blob(name_w, name_a));
        wrap = wrap.py(fz(2.)).child(line().pl(fz(80.)).child(stamp).child(name).child(content));
    } else {
        wrap = wrap.pl(fz(72.));
        if group_start {
            let side = (40. * scale).min(40.);
            wrap = wrap
                .child(div().absolute().left(fz(16.)).top(fz(4.)).size(z(side)).rounded_full().bg(fill(0.08)))
                .child(line().child(blob(name_w, name_a)));
        }
        wrap = wrap.child(line().child(content));
    }
    if let Some(a) = attachment {
        let att = div().w(px(a.width)).h(px(a.height)).mt(fz(2.)).rounded(px(6.)).bg(fill(0.03));
        wrap = wrap.child(div().py(fz(2.)).when(compact, |d| d.mt(fz(4.)).pl(fz(80.))).child(att));
    }
    wrap.into_any_element()
}

fn group(rng: &mut Rng, compact: bool, count: u32, spacing: f32, attachment: Option<Attachment>) -> Div {
    let at = attachment.map(|_| rng.int(0, count - 1));
    let name_w = rng.int(80, 120) as f32;
    let name_a = rng.float(0.1, 0.2);
    div().flex_none().flex().flex_col().children((0..count).map(|i| {
        let top = if i == 0 { spacing * font_scale() } else { 0. };
        message(rng, compact, i == 0, name_w, name_a, attachment.filter(|_| at == Some(i)), top)
    }))
}

pub fn render(seed: u64, window_height: f32, compact: bool, group_spacing: f32) -> Div {
    let specs = generate(seed, window_height, compact, font_scale(), group_spacing);
    let mut rng = Rng::new(seed.rotate_left(17));
    let mut col = div().w_full().flex_none().h(z(specs.total_height)).overflow_hidden().flex().flex_col();
    for (i, &n) in specs.groups.iter().enumerate() {
        let att = specs.attachments.iter().find(|(g, _)| *g == i).map(|(_, a)| *a);
        col = col.child(group(&mut rng, compact, n, specs.group_spacing, att));
    }
    col
}

static WINDOW_H: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(720);

pub fn set_window_height(h: f32) {
    WINDOW_H.store(h.max(0.) as u32, std::sync::atomic::Ordering::Relaxed);
}

pub fn window_height() -> f32 {
    WINDOW_H.load(std::sync::atomic::Ordering::Relaxed) as f32
}

pub fn for_channel(channel: u64, compact: bool) -> Div {
    render(channel, window_height(), compact, group_spacing(compact))
}

pub fn group_spacing(compact: bool) -> f32 {
    if compact { 0. } else { 16. }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_follows_the_client_formula() {
        assert_eq!(page_messages(720., false, 1.), 10);
        assert_eq!(page_messages(1440., false, 1.), 16);
        assert_eq!(page_messages(1440., true, 1.), 21);
        assert_eq!(page_messages(4000., true, 1.), 30);
    }

    #[test]
    fn specs_are_stable_per_seed_and_inside_the_client_ranges() {
        let a = generate(42, 1000., false, 1., 16.);
        assert_eq!(a, generate(42, 1000., false, 1., 16.));
        assert_eq!(a.groups.len(), (0.87 * page_messages(1000., false, 1.) as f32).round() as usize);
        assert!(a.groups.iter().all(|&n| (1..=4).contains(&n)));
        assert!(!a.attachments.is_empty());
        for (_, at) in &a.attachments {
            assert!((140. ..=400.).contains(&at.width) && (100. ..=320.).contains(&at.height));
        }
        let expect: f32 = a.groups.iter().map(|&n| 16. + 44. + (n - 1) as f32 * 22.).sum::<f32>()
            + a.attachments.iter().map(|(_, at)| at.height + 6.).sum::<f32>();
        assert!((a.total_height - expect).abs() < 0.01);
    }

    #[test]
    fn rng_stays_in_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            assert!((1..=4).contains(&r.int(1, 4)));
            let f = r.float(0.02, 0.08);
            assert!((0.02..=0.08).contains(&f));
        }
    }
}
