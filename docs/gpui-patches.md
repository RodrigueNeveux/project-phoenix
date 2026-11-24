# Patches to GPUI (`gpui-pre` 0.3.12)

`vendor/gpui-pre` is the crates.io `gpui-pre` 0.3.12 source, wired in through
`[patch.crates-io]` in the workspace `Cargo.toml` (and excluded from the workspace
members). Examples, benches, tests, docs and the lockfile were removed from the copy,
with their `[[example]]`/`[[bench]]`/`[[test]]` tables, to keep it small. The library
source is untouched except for the patches listed here. Each one is marked
`PHOENIX PATCH` in the code.

To upgrade GPUI: copy the new release into `vendor/gpui-pre`, re-apply the patches below,
and bump the `=0.3.12` pins in `crates/phoenix-app/Cargo.toml`.

## 1. Subpixel layout (`src/taffy.rs`, `impl ToTaffy<f32> for AbsoluteLength`)

**Problem.** Upstream rounds every authored length (padding, margin, gap, width, height)
to whole device pixels *before* layout. At a fractional scale such as 125 % this makes
lengths drift: a 38px channel row (1 + 36 + 1) is laid out as 1 + 45 + 1 = 47 device px
instead of 47.5. The official client (Chromium) keeps layout fractional and snaps only
the painted edges. So our lists drifted by 0.5 dp per row: about 3 px after 6 rows in the
channel list, with the same drift in the guild rail (measured with `scripts/compare`
against the live client at DPR 1.25).

**Patch.** `AbsoluteLength::to_taffy` returns the exact device-pixel value
(`logical * scale_factor`) instead of `round_half_toward_zero(logical * scale_factor)`.
The rest of upstream's snapping is kept:
- `layout_bounds` still rounds each element's outer edges in absolute device pixels after
  layout. That is one snapping step at "paint", and coincident edges still close.
- Border widths still go through `round_stroke_to_device_pixel`, so a 1px border stays
  1 dp and is never blurred.
- Measured text/leaf sizes are still ceiled to device pixels.

At integer scale factors (1x, 2x) authored integer lengths are already whole device
pixels, so the output there is unchanged.

## 2. Fractional measured sizes (`src/element.rs` `Drawable::layout_as_root`, `src/window.rs`, `src/taffy.rs`)

**Problem.** `list` measures each item as its own layout root and stacks the measured
heights. The measured size was the snapped one, and the far edge of a root at origin 0
rounds half toward zero (47.5 becomes 47), so with patch 1 alone the channel list still
lost 0.5 dp per row.

**Patch.** `layout_as_root` returns the raw taffy size (new
`Window::layout_size_unsnapped` / `TaffyLayoutEngine::layout_size_unsnapped`). Items are
then placed at fractional offsets, and `layout_bounds` snaps their edges in absolute
device pixels at paint, so they close with no drift.

## Effect (live compare, DPR 1.25, same screen)

| region | upstream | patched |
|---|---|---|
| guild rail | 14.2 % | 2.6 % (patch 1) |
| member list | 16.0 % | 12.3 % (patch 1) |
| messages | 6.8 % | 5.4 % (patch 1) |
| overall | 8.1 % | 6.7 % (patch 1) |
