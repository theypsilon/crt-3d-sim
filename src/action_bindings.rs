use crate::console;
use crate::simulation_state::Input;

pub fn on_button_action(input: &mut Input, button_action: &str, pressed: bool) {
    match button_action {
        "," => {
            if !input.input_focused {
                input.next_layering_kind.input = pressed
            }
        }
        "." => {
            if !input.input_focused {
                input.toggle_pixels_shadow_kind.input = pressed
            }
        }
        "feature-change-screen-layering-type" => input.next_layering_kind.input = pressed,
        "feature-change-pixel-shadow" => input.toggle_pixels_shadow_kind.input = pressed,
        "+" => {
            if !input.input_focused {
                input.rotate_left = pressed
            }
        }
        "-" => {
            if !input.input_focused {
                input.rotate_right = pressed
            }
        }
        "input_focused" => input.input_focused = pressed,
        "a" => input.walk_left = pressed,
        "d" => input.walk_right = pressed,
        "w" => input.walk_forward = pressed,
        "s" => input.walk_backward = pressed,
        "q" => input.walk_up = pressed,
        "e" => input.walk_down = pressed,
        "arrowleft" | "←" | "◀" => input.turn_left = pressed,
        "arrowright" | "→" | "▶" => input.turn_right = pressed,
        "arrowup" | "↑" | "▲" => input.turn_up = pressed,
        "arrowdown" | "↓" | "▼" => input.turn_down = pressed,
        "f" => {
            if input.shift {
                input.filter_speed.increase.input = pressed
            } else {
                input.translation_speed.increase.input = pressed
            }
        }
        "r" => {
            if input.shift {
                input.filter_speed.decrease.input = pressed
            } else {
                input.translation_speed.decrease.input = pressed
            }
        }
        "feature-change-move-speed-inc" => input.translation_speed.increase.input = pressed,
        "feature-change-move-speed-dec" => input.translation_speed.decrease.input = pressed,
        "feature-change-pixel-speed-inc" => input.filter_speed.increase.input = pressed,
        "feature-change-pixel-speed-dec" => input.filter_speed.decrease.input = pressed,
        "t" | "reset-speeds" => input.reset_speeds = pressed,
        "camera-zoom-inc" => input.camera_zoom.increase = pressed,
        "camera-zoom-dec" => input.camera_zoom.decrease = pressed,
        "u" | "pixel-vertical-gap-inc" => input.pixel_scale_x.increase = pressed,
        "i" | "pixel-vertical-gap-dec" => input.pixel_scale_x.decrease = pressed,
        "j" | "pixel-horizontal-gap-inc" => input.pixel_scale_y.increase = pressed,
        "k" | "pixel-horizontal-gap-dec" => input.pixel_scale_y.decrease = pressed,
        "n" | "pixel-width-inc" => {
            if input.shift {
                input.pixel_gap.increase = pressed
            } else {
                input.pixel_width.increase = pressed
            }
        }
        "m" | "pixel-width-dec" => {
            if input.shift {
                input.pixel_gap.decrease = pressed
            } else {
                input.pixel_width.decrease = pressed
            }
        }
        "b" | "blur-level-inc" => input.blur.increase.input = pressed,
        "v" | "bluer-level-dec" => input.blur.decrease.input = pressed,
        "<" | "&lt;" | "pixel-contrast-inc" => input.contrast.increase = pressed,
        "z" | "pixel-contrast-dec" => input.contrast.decrease = pressed,
        "c" | "pixel-brightness-inc" => input.bright.increase = pressed,
        "x" | "pixel-brightness-dec" => input.bright.decrease = pressed,
        "y" | "feature-change-color-representation" => input.next_color_representation_kind.input = pressed,
        "o" | "feature-change-pixel-geometry" => input.next_pixel_geometry_kind.input = pressed,
        "l" | "feature-change-screen-curvature" => input.next_screen_curvature_type.input = pressed,
        "g" | "lines-per-pixel-inc" => input.lpp.increase.input = pressed,
        "h" | "lines-per-pixel-dec" => input.lpp.decrease.input = pressed,
        "shift" => {
            input.shift = pressed;
            if input.shift {
                input.pixel_width.increase = false;
                input.pixel_width.decrease = false
            } else {
                input.pixel_gap.increase = false;
                input.pixel_gap.decrease = false
            }
        }
        "alt" => input.alt = pressed,
        " " | "space" => input.space.input = pressed,
        "escape" | "esc" | "feature-quit" => input.esc.input = pressed,
        "f4" => input.screenshot.input = pressed,
        "reset-camera" => input.reset_position = pressed,
        "reset-filters" => input.reset_filters = pressed,
        _ => {
            if button_action.contains('+') {
                for button_fraction in button_action.split('+') {
                    on_button_action(input, button_fraction, pressed);
                }
            } else if pressed {
                console!(log. "Ignored key: ", button_action);
            }
        }
    }
}
