"""Exercise native_text_scaled without a game window."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from test_world_input_capture import method

ROOT = Path(__file__).resolve().parents[1]


class NativeTextScaledTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("rustc"), "requires Rust")
    def test_text_is_scaled_about_its_origin_and_alpha_is_kept(self):
        source = (ROOT / "crates/horizon-client/src/game/native_hooks.rs").read_text(encoding="utf-8")
        actual = method(source.replace("pub(super) fn", "fn"), "native_text_scaled")
        fixture = r"""
struct Text;
impl Text { fn literal(_: &str) -> Text { Text } }
fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 { u32::from_le_bytes([r, g, b, a]) }
struct Atlas;
struct Gui { atlas: Atlas }
struct HudBatch { vertices: Vec<u32>, colors: Vec<u32>, scaled: Option<(usize, f32, f32, f32, f32)> }
impl HudBatch {
    fn text_styled(&mut self, _: &Atlas, _: &Text, _x: f32, _y: f32, color: u32, _s: bool) -> f32 {
        self.vertices.push(7);
        self.colors.push(color);
        20.0
    }
    fn scale_since(&mut self, start: usize, x: f32, y: f32, sx: f32, sy: f32) {
        self.scaled = Some((start, x, y, sx, sy));
    }
}
struct Game { gui: Gui }
impl Game {
    METHOD
}
fn batch() -> HudBatch { HudBatch { vertices: vec![0], colors: vec![], scaled: None } }
#[test]
fn scales_what_it_drew_about_the_origin() {
    let game = Game { gui: Gui { atlas: Atlas } };
    let mut b = batch();
    let width = game.native_text_scaled(&mut b, "hi", [4.0, 5.0], 1.5, 0x80FF_8000, true);
    assert_eq!(width, 30.0);
    assert_eq!(b.vertices, vec![0, 7]);
    assert_eq!(b.scaled, Some((1, 4.0, 5.0, 1.5, 1.5)));
    assert_eq!(b.colors, vec![rgba(0xFF, 0x80, 0x00, 0x80)]);
}
#[test]
fn a_plain_rgb_is_opaque() {
    let game = Game { gui: Gui { atlas: Atlas } };
    let mut b = batch();
    game.native_text_scaled(&mut b, "hi", [0.0, 0.0], 2.0, 0x00FF_FFFF, false);
    assert_eq!(b.colors, vec![rgba(0xFF, 0xFF, 0xFF, 0xFF)]);
}
#[test]
fn a_bad_scale_draws_nothing() {
    let game = Game { gui: Gui { atlas: Atlas } };
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut b = batch();
        assert_eq!(game.native_text_scaled(&mut b, "hi", [0.0, 0.0], scale, 0xFFFF_FFFF, true), 0.0);
        assert_eq!(b.vertices, vec![0]);
        assert_eq!(b.scaled, None);
    }
}
""".replace("METHOD", actual)
        with tempfile.TemporaryDirectory() as folder:
            src, exe = Path(folder) / "text.rs", Path(folder) / "text.exe"
            src.write_text(fixture, encoding="utf-8")
            subprocess.run(["rustc", "--edition=2024", "--test", str(src), "-o", str(exe)], check=True)
            subprocess.run([str(exe)], check=True)
