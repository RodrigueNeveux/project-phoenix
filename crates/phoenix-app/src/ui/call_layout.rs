pub const RATIO: f64 = 16. / 9.;
pub const GRID_PAD_Y: f32 = 64.;
pub const STRIP_TILE_W: f32 = 195.;
pub const STRIP_TILE_H: f32 = 112.;
pub const STRIP_GAP: f32 = 8.;
const STRIP_MIN_W: f64 = 124.;
const STRIP_LIMIT: usize = 12;
const MAX_TILES: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub tile_w: f32,
    pub pad_column: bool,
    pub top: f32,
}

impl Grid {
    pub fn tile_h(&self) -> f32 {
        self.tile_w * 9. / 16.
    }

    pub fn last_column(&self, i: usize, n: usize) -> bool {
        (i + 1) % self.columns == 0 || i + 1 == n
    }

    pub fn last_row(&self, i: usize) -> bool {
        i >= (self.rows - 1) * self.columns
    }
}

fn columns(tile: f64, width: f64) -> usize {
    (((width - tile) / (8. + tile)).floor() + 1.).max(1.) as usize
}

fn search(mut lo: f64, hi: f64, pred: impl Fn(f64) -> bool) -> f64 {
    let mut span = hi - lo;
    while span > 0. {
        let half = (span / 2.).floor();
        let mid = lo + half;
        if pred(mid) {
            lo = mid + 1.;
            span -= half + 1.;
        } else {
            span = half;
        }
    }
    lo
}

pub fn grid(n: usize, width: f32, height: f32) -> Grid {
    let n = n.max(1);
    let (width, height) = (width as f64, height as f64);
    let w = width - 16.;
    let h = height - 2. * GRID_PAD_Y as f64;
    let lo = (w / 25.).floor();
    let tile = if n > MAX_TILES {
        search(lo, w, |e| {
            let th = e / RATIO;
            columns(e, w) as f64 * (((h - th) / (8. + th)).ceil() + 1.) > MAX_TILES as f64
        })
    } else {
        search(lo, w, |e| {
            let th = e / RATIO;
            columns(e, w) as f64 * (((h - th) / (8. + th)).floor() + 1.) >= n as f64
        }) - 1.
    };
    let cols = columns(tile, w).max(1);
    let rows = n.div_ceil(cols);
    let next = (cols + 1) as f64;
    let pad_column = next * tile + (next - 1.) * 8. <= width;
    let row_h = (tile / RATIO).floor() + 8.;
    let spare = (h - row_h * rows as f64).max(0.) / 2.;
    Grid { columns: cols, rows, tile_w: tile as f32, pad_column, top: GRID_PAD_Y + spare as f32 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    pub top: f32,
    pub count: usize,
    pub tile_w: f32,
    pub rule_w: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Focus {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub action_top: f32,
    pub strip: Option<Strip>,
}

pub fn strip(width: f32, others: usize) -> (usize, f32, f32) {
    let m = width as f64;
    let max_w = RATIO * STRIP_TILE_H as f64 + 8.;
    let count = (((m - 8.) / (STRIP_MIN_W + 8.)).floor().max(0.) as usize).min(STRIP_LIMIT).min(others);
    if count == 0 {
        return (0, 0., 0.);
    }
    let rule = ((m - 8.) / count as f64 - 8.).min(max_w);
    let inner = width - 2. * STRIP_GAP - STRIP_GAP * (count - 1) as f32;
    let shown = (inner / count as f32).min(STRIP_TILE_W);
    (count, shown, rule as f32)
}

pub fn focus(width: f32, height: f32, aspect: f32, strip_open: bool, others: usize) -> Focus {
    let shown = strip_open && height > 384. && others > 0;
    let edge = if shown { 72. + 68. } else { 0. };
    let band = (height - 2. * edge).max(0.);
    let w = width.min(band * aspect);
    let h = w / aspect;
    let lift = if shown { 60. } else { 0. };
    let y = (height - h) / 2. - lift;
    let x = (width - w) / 2.;
    let eo = (width.min(band * aspect) / aspect).floor();
    let action_bottom = if shown { -16. } else { 40. + (72. - (height - eo) / 2.).max(0.) };
    let strip = shown.then(|| {
        let (count, tile_w, rule_w) = strip(width, others);
        Strip { top: (height + h) / 2. + lift - STRIP_TILE_H, count, tile_w, rule_w }
    });
    Focus { x, y, w, h, action_top: y + h - action_bottom, strip }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileLook {
    pub avatar: f32,
    pub show_name: bool,
    pub compact: bool,
    pub small: bool,
    pub inset: f32,
    pub name_size: (f32, f32),
}

pub fn tile_look(width: f32) -> TileLook {
    let compact = width <= 220.;
    TileLook {
        avatar: if width < 124. { 40. } else { 80. },
        show_name: width > 124.,
        compact,
        small: width < 195.,
        inset: if compact { 4. } else { 12. },
        name_size: if compact { (14., 18.) } else { (16., 20.) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.11
    }

    #[test]
    fn solo_call_matches_the_client_dump() {
        let g = grid(2, 1119.4, 807.2);
        assert_eq!((g.columns, g.rows), (1, 2));
        assert_eq!(g.tile_w, 596.);
        assert!(near(g.top, 64.));
        assert!(!g.pad_column);
        assert!(near(g.tile_h(), 335.25));
    }

    #[test]
    fn grid_fills_rows_for_one_to_twelve() {
        let sizes = [(1119.4, 807.2), (864., 667.), (1600., 900.), (600., 500.)];
        for (w, h) in sizes {
            let mut last = f32::MAX;
            for n in 1..=12 {
                let g = grid(n, w, h);
                assert!(g.columns * g.rows >= n, "{n} {w}x{h}");
                assert!(g.rows == n.div_ceil(g.columns));
                let used = g.columns as f32 * g.tile_w + (g.columns - 1) as f32 * 8.;
                assert!(used <= w - 16. + 0.01, "{n} {w}x{h}");
                let tall = g.rows as f32 * ((g.tile_w as f64 / RATIO).floor() as f32 + 8.) - 8.;
                assert!(tall <= h - 128. + 0.01, "{n} {w}x{h}");
                assert!(g.tile_w <= last + 0.01);
                last = g.tile_w;
            }
        }
    }

    #[test]
    fn grid_columns_follow_the_client_search() {
        let g = grid(1, 1119.4, 807.2);
        assert_eq!((g.columns, g.tile_w), (1, 1103.));
        assert!(near(g.top, 89.6));
        assert_eq!(grid(4, 1600., 900.), Grid { columns: 2, rows: 2, tile_w: 679., pad_column: false, top: 64. });
        assert_eq!((grid(6, 1600., 600.).columns, grid(6, 1600., 600.).tile_w), (3, 412.));
        let g = grid(12, 1600., 900.);
        assert_eq!((g.columns, g.rows, g.tile_w), (4, 3, 390.));
        assert!(near(g.top, 109.5));
        assert_eq!(grid(2, 2000., 400.).columns, 4);
        let g = grid(3, 985., 500.);
        assert_eq!((g.columns, g.rows, g.tile_w, g.pad_column), (2, 2, 323., true));
        assert!(g.last_column(1, 3) && !g.last_column(0, 3) && g.last_column(2, 3));
        assert!(g.last_row(2) && !g.last_row(1));
    }

    #[test]
    fn many_tiles_cap_the_visible_grid() {
        let g = grid(40, 1600., 900.);
        assert_eq!((g.columns, g.rows, g.tile_w), (5, 8, 264.));
    }

    #[test]
    fn focus_matches_the_client_dump() {
        let f = focus(864., 667., 684. / 387., true, 6);
        assert!(near(f.h, 387.));
        assert!(near(f.w, 684.));
        assert!(near(f.x, 90.));
        assert!(near(f.y, 80.));
        let s = f.strip.unwrap();
        assert!(near(s.top, 475.));
        assert_eq!(s.count, 6);
        assert!((s.tile_w - 134.67).abs() < 0.1);
        assert!(near(f.action_top, 483.));
    }

    #[test]
    fn focus_without_strip_uses_the_full_height() {
        let f = focus(1600., 900., 16. / 9., false, 3);
        assert!(f.strip.is_none());
        assert!(near(f.h, 900.));
        let f = focus(864., 380., 16. / 9., true, 3);
        assert!(f.strip.is_none());
        let f = focus(400., 800., 16. / 9., true, 3);
        assert!(near(f.w, 400.));
        assert!(near(f.h, 225.));
    }

    #[test]
    fn strip_shrinks_and_caps() {
        assert_eq!(strip(1600., 3), (3, STRIP_TILE_W, (RATIO * 112. + 8.) as f32));
        let (n, w, _) = strip(864., 20);
        assert_eq!(n, 6);
        assert!(w < STRIP_TILE_W);
        assert_eq!(strip(3000., 20).0, 12);
        assert_eq!(strip(100., 5).0, 0);
    }

    #[test]
    fn tile_look_follows_the_client_breakpoints() {
        let big = tile_look(596.);
        assert_eq!((big.avatar, big.show_name, big.compact, big.inset, big.name_size), (80., true, false, 12., (16., 20.)));
        let mid = tile_look(200.);
        assert!(mid.compact && mid.small && mid.show_name && mid.inset == 4.);
        let tiny = tile_look(120.);
        assert_eq!((tiny.avatar, tiny.show_name), (40., false));
        assert!(!tile_look(124.).show_name && tile_look(124.).avatar == 80.);
    }
}
