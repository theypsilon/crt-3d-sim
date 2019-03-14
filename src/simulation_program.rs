use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use web_sys::{
    Window,
    WebGl2RenderingContext,
    WebGlFramebuffer,
};
use std::rc::Rc;

use crate::wasm_error::{WasmResult, WasmError};
use crate::camera::{CameraDirection, Camera};
use crate::dispatch_event::{dispatch_event, dispatch_event_with};
use crate::web_utils::{now, window};
use crate::pixels_render::{PixelsRender, PixelsGeometryKind, PixelsUniform};
use crate::blur_render::{BlurRender};
use crate::internal_resolution_render::InternalResolutionRender;
use crate::rgb_render::RgbRender;
use crate::background_render::BackgroundRender;
use crate::event_listeners::{set_event_listeners};
use crate::simulation_state::{
    StateOwner, Resources, CrtFilters, SimulationTimers, InitialParameters, ColorChannels, RenderLayers,
    Input, AnimationData
};
use crate::render_types::{TextureBufferStack};
use crate::action_bindings::on_button_action;
use crate::console;

const PIXEL_MANIPULATION_BASE_SPEED: f32 = 20.0;
const TURNING_BASE_SPEED: f32 = 3.0;
const MOVEMENT_BASE_SPEED: f32 = 10.0;
const MOVEMENT_SPEED_FACTOR: f32 = 50.0;

pub fn program(gl: JsValue, animation: AnimationData) -> WasmResult<()> {
    let gl = gl.dyn_into::<WebGl2RenderingContext>()?;
    let owned_state = StateOwner::new_rc(load_resources(&gl, animation)?, Input::new()?);
    let frame_closure: Closure<FnMut(JsValue)> = {
        let owned_state = Rc::clone(&owned_state);
        let window = window()?;
        Closure::wrap(Box::new(move |_| {
            if let Err(e) = program_iteration(&owned_state, &gl, &window) {
                console!(error. "An unexpected error happened during program_iteration.", e.to_js());
            }
        }))
    };
    window()?.request_animation_frame(frame_closure.as_ref().unchecked_ref())?;
    let mut closures = owned_state.closures.borrow_mut();
    closures.push(Some(frame_closure));

    let listeners = set_event_listeners(&owned_state)?;
    closures.extend(listeners);

    Ok(())
}

fn program_iteration(owned_state: &StateOwner, gl: &WebGl2RenderingContext, window: &Window) -> WasmResult<()> {
    let mut input = owned_state.input.borrow_mut();
    let mut resources = owned_state.resources.borrow_mut();
    let closures = owned_state.closures.borrow();
    pre_process_input(&mut input, &resources)?;
    if !update_simulation(&mut resources, &input)? { 
        console!(log. "User closed the simulation.");
        return Ok(());
    }
    post_process_input(&mut input)?;
    draw(&gl, &resources)?;
    window.request_animation_frame(
        closures[0].as_ref().ok_or("Wrong closure.")?
        .as_ref().unchecked_ref()
    )?;
    Ok(())
}

fn load_resources(gl: &WebGl2RenderingContext, animation: AnimationData) -> WasmResult<Resources> {
    let initial_position_z = calculate_far_away_position(&animation);
    let mut camera = Camera::new(MOVEMENT_BASE_SPEED * initial_position_z / MOVEMENT_SPEED_FACTOR, TURNING_BASE_SPEED);
    camera.set_position(glm::vec3(0.0, 0.0, initial_position_z));
    let mut crt_filters = CrtFilters::new(PIXEL_MANIPULATION_BASE_SPEED);
    crt_filters.cur_pixel_width = animation.pixel_width;

    let internal_resolution_multiplier : i32 = 2;
    let internal_width = animation.viewport_width as i32 * internal_resolution_multiplier;
    let internal_height = animation.viewport_height as i32 * internal_resolution_multiplier;

    let now = now()?;
    let res = Resources {
        internal_resolution_multiplier,
        initial_parameters: InitialParameters {
            initial_position_z,
            initial_pixel_width: animation.pixel_width,
            initial_movement_speed: camera.movement_speed,
        },
        timers: SimulationTimers {
            frame_count: 0,
            last_time: now,
            last_second: now,
        },
        pixels_render: PixelsRender::new(&gl, animation.image_width as usize, animation.image_height as usize)?,
        blur_render: BlurRender::new(&gl)?,
        internal_resolution_render: InternalResolutionRender::new(gl)?,
        rgb_render: RgbRender::new(gl)?,
        background_render: BackgroundRender::new(gl)?,
        texture_buffer_stack: std::cell::RefCell::new(TextureBufferStack::new(internal_width, internal_height)),
        animation,
        camera,
        crt_filters,
        launch_screenshot: false,
    };
    change_frontend_input_values(&res)?;
    Ok(res)
}

fn change_frontend_input_values(res: &Resources) -> WasmResult<()> {
    dispatch_event_with("app-event.change_pixel_horizontal_gap", &res.crt_filters.cur_pixel_scale_x.into())?;
    dispatch_event_with("app-event.change_pixel_vertical_gap", &res.crt_filters.cur_pixel_scale_y.into())?;
    dispatch_event_with("app-event.change_pixel_width", &res.crt_filters.cur_pixel_width.into())?;
    dispatch_event_with("app-event.change_pixel_spread", &res.crt_filters.cur_pixel_gap.into())?;
    dispatch_event_with("app-event.change_pixel_brightness", &res.crt_filters.extra_bright.into())?;
    dispatch_event_with("app-event.change_pixel_contrast", &res.crt_filters.extra_contrast.into())?;
    dispatch_event_with("app-event.change_light_color", &res.crt_filters.light_color.into())?;
    dispatch_event_with("app-event.change_brightness_color", &res.crt_filters.brightness_color.into())?;
    dispatch_event_with("app-event.change_camera_zoom", &res.camera.zoom.into())?;
    dispatch_event_with("app-event.change_blur_level", &(res.crt_filters.blur_passes as i32).into())?;
    dispatch_event_with("app-event.change_lines_per_pixel", &(res.crt_filters.lines_per_pixel as i32).into())?;
    dispatch_event_with("app-event.change_movement_speed", &((res.camera.movement_speed / res.initial_parameters.initial_movement_speed) as i32).into())?;
    dispatch_event_with("app-event.change_pixel_speed", &((res.crt_filters.change_speed / PIXEL_MANIPULATION_BASE_SPEED) as i32).into())?;
    dispatch_event_with("app-event.change_turning_speed", &((res.camera.turning_speed / TURNING_BASE_SPEED) as i32).into())?;
    Ok(())
}

fn calculate_far_away_position(animation: &AnimationData) -> f32 {
    let width = animation.background_width as f32;
    let height = animation.background_height as f32;
    let viewport_width_scaled = (animation.viewport_width as f32 / animation.pixel_width) as u32;
    let width_ratio = viewport_width_scaled as f32 / width;
    let height_ratio = animation.viewport_height as f32 / height;
    let is_height_bounded = width_ratio > height_ratio;
    let mut bound_ratio = if is_height_bounded {height_ratio} else {width_ratio};
    let mut resolution = if is_height_bounded {animation.viewport_height} else {viewport_width_scaled} as i32;
    while bound_ratio < 1.0 {
        bound_ratio *= 2.0;
        resolution *= 2;
    }
    if !animation.stretch {
        let mut divisor = bound_ratio as i32;
        while divisor > 1 {
            if resolution % divisor == 0 {
                break;
            }
            divisor -= 1;
        };
        bound_ratio = divisor as f32;
    }
    0.5 + (resolution as f32 / bound_ratio) * if is_height_bounded {1.2076} else {0.68 * animation.pixel_width}
}

fn pre_process_input(input: &mut Input, resources: &Resources) -> WasmResult<()> {
    input.now = now().unwrap_or(resources.timers.last_time);
    input.toggle_pixels_shadow_kind.track_input();
    input.speed_up.track_input();
    input.speed_down.track_input();
    input.mouse_click.track_input();
    input.increase_blur.track_input();
    input.decrease_blur.track_input();
    input.increase_lpp.track_input();
    input.decrease_lpp.track_input();
    input.next_color_representation_kind.track_input();
    input.next_pixel_geometry_kind.track_input();
    input.next_layering_kind.track_input();
    input.showing_pixels_pulse.track_input();
    input.esc.track_input();
    input.space.track_input();
    input.screenshot.track_input();
    Ok(())
}

fn post_process_input(input: &mut Input) -> WasmResult<()> {
    input.mouse_scroll_y = 0.0;
    input.mouse_position_x = 0;
    input.mouse_position_y = 0;
    input.custom_event.kind = String::new();
    Ok(())
}

fn update_simulation(res: &mut Resources, input: &Input) -> WasmResult<bool> {
    let dt = update_timers_and_dt(res, input)?;
    
    update_animation_buffer(res, input);
    update_colors(dt, res, input)?;
    update_blur(res, input)?;
    update_lpp(res, input)?;

    if input.esc.is_just_pressed() {
        dispatch_event("app-event.exiting_session")?;
        return Ok(false);
    }

    if input.space.is_just_pressed() {
        dispatch_event("app-event.toggle_info_panel")?;
    }


    update_pixel_pulse(dt, res, input)?;
    update_crt_filters(dt, res, input)?;
    update_speeds(res, input)?;
    update_camera(dt, res, input)?;
    res.launch_screenshot = input.screenshot.is_just_released();

    Ok(true)
}

fn update_timers_and_dt(res: &mut Resources, input: &Input) -> WasmResult<f32> {
    let dt: f32 = ((input.now - res.timers.last_time) / 1000.0) as f32;
    let ellapsed = input.now - res.timers.last_second;
    res.timers.last_time = input.now;

    if ellapsed >= 1_000.0 {
        let fps = res.timers.frame_count as f32;
        dispatch_event_with("app-event.fps", &fps.into())?;
        res.timers.last_second = input.now;
        res.timers.frame_count = 0;
    } else {
        res.timers.frame_count += 1;
    }
    Ok(dt)
}

fn update_animation_buffer(res: &mut Resources, input: &Input) {
    res.animation.needs_buffer_data_load = false;
    let next_frame_update = res.animation.last_frame_change + f64::from(res.animation.frame_length);
    if input.now >= next_frame_update {
        res.animation.last_frame_change = next_frame_update;
        let last_frame = res.animation.current_frame;
        res.animation.current_frame += 1;
        if res.animation.current_frame >= res.animation.steps.len() {
            res.animation.current_frame = 0;
        }
        if last_frame != res.animation.current_frame {
            res.animation.needs_buffer_data_load = true;
        }
    }
}

fn update_colors(dt: f32, res: &mut Resources, input: &Input) -> WasmResult<()> {
    if input.increase_bright {
        res.crt_filters.extra_bright += 0.01 * dt * res.crt_filters.change_speed;
    }
    if input.decrease_bright {
        res.crt_filters.extra_bright -= 0.01 * dt * res.crt_filters.change_speed;
    }
    if input.increase_bright || input.decrease_bright {
        if res.crt_filters.extra_bright < -1.0 {
            res.crt_filters.extra_bright = -1.0;
            dispatch_event_with("app-event.top_message", &"Minimum value is -1.0".into())?;
        } else if res.crt_filters.extra_bright > 1.0 {
            res.crt_filters.extra_bright = 1.0;
            dispatch_event_with("app-event.top_message", &"Maximum value is +1.0".into())?;
        } else {
            dispatch_event_with("app-event.change_pixel_brightness", &res.crt_filters.extra_bright.into())?;
        }
    }
    if input.increase_contrast {
        res.crt_filters.extra_contrast += 0.01 * dt * res.crt_filters.change_speed;
    }
    if input.decrease_contrast {
        res.crt_filters.extra_contrast -= 0.01 * dt * res.crt_filters.change_speed;
    }
    if input.increase_contrast || input.decrease_contrast {
        if res.crt_filters.extra_contrast < 0.0 {
            res.crt_filters.extra_contrast = 0.0;
            dispatch_event_with("app-event.top_message", &"Minimum value is 0.0".into())?;
        } else if res.crt_filters.extra_contrast > 20.0 {
            res.crt_filters.extra_contrast = 20.0;
            dispatch_event_with("app-event.top_message", &"Maximum value is 20.0".into())?;
        } else {
            dispatch_event_with("app-event.change_pixel_contrast", &res.crt_filters.extra_contrast.into())?;
        }
    }
    let color_variable = match input.custom_event.kind.as_ref() {
        "event_kind:pixel_brightness" => {
            res.crt_filters.extra_bright = input.custom_event.value.as_f64().ok_or("it should be a number")? as f32;
            return Ok(());
        },
        "event_kind:pixel_contrast" => {
            res.crt_filters.extra_contrast = input.custom_event.value.as_f64().ok_or("it should be a number")? as f32;
            return Ok(());
        },
        "event_kind:light_color" => &mut res.crt_filters.light_color,
        "event_kind:brightness_color" => &mut res.crt_filters.brightness_color,
        _ => return Ok(()),
    };

    let color_pick = input.custom_event.value.as_f64().ok_or("it should be a number")? as i32;
    if color_pick != *color_variable {
        *color_variable = color_pick;
        dispatch_event_with("app-event.top_message", &"Color changed.".into())?;
    }
    
    Ok(())
}

fn update_blur(res: &mut Resources, input: &Input) -> WasmResult<()> {
    let last_blur_passes = res.crt_filters.blur_passes;
    if input.increase_blur.is_just_pressed() {
        res.crt_filters.blur_passes += 1;
    }
    if input.decrease_blur.is_just_pressed() {
        if res.crt_filters.blur_passes > 0 {
            res.crt_filters.blur_passes -= 1;
        } else {
            dispatch_event_with("app-event.top_message", &"Minimum value is 0".into())?;
        }
    }
    if input.custom_event.kind.as_ref() as &str == "event_kind:blur_level" {
        res.crt_filters.blur_passes = input.custom_event.value.as_f64().ok_or("it should be a number")? as usize;
        dispatch_event_with("app-event.change_blur_level", &(res.crt_filters.blur_passes as i32).into())?;
    }
    if res.crt_filters.blur_passes > 100 {
        res.crt_filters.blur_passes = 100;
        dispatch_event_with("app-event.top_message", &"Maximum value is 100".into())?;
    }
    if last_blur_passes != res.crt_filters.blur_passes {
        console!(log. "blur_level changed!");
        dispatch_event_with("app-event.top_message", &("Blur level: ".to_string() + &res.crt_filters.blur_passes.to_string()).into())?;
        dispatch_event_with("app-event.change_blur_level", &(res.crt_filters.blur_passes as i32).into())?;
    }
    Ok(())
}

// lines per pixel
fn update_lpp(res: &mut Resources, input: &Input) -> WasmResult<()> {
    let last_lpp = res.crt_filters.lines_per_pixel;
    if input.increase_lpp.is_just_pressed() {
        res.crt_filters.lines_per_pixel += 1;
    }
    if input.decrease_lpp.is_just_pressed() && res.crt_filters.lines_per_pixel > 0 {
        res.crt_filters.lines_per_pixel -= 1;
    }
    if input.custom_event.kind.as_ref() as &str == "event_kind:lines_per_pixel" {
        res.crt_filters.lines_per_pixel = input.custom_event.value.as_f64().ok_or("it should be a number")? as usize;
        dispatch_event_with("app-event.change_lines_per_pixel", &(res.crt_filters.lines_per_pixel as i32).into())?;
    }
    if res.crt_filters.lines_per_pixel < 1 {
        res.crt_filters.lines_per_pixel = 1;
        dispatch_event_with("app-event.top_message", &"Minimum value is 1".into())?;
    } else if res.crt_filters.lines_per_pixel > 20 {
        res.crt_filters.lines_per_pixel = 20;
        dispatch_event_with("app-event.top_message", &"Maximum value is 20".into())?;
    }
    if last_lpp != res.crt_filters.lines_per_pixel {
        dispatch_event_with("app-event.top_message", &("Lines per pixel: ".to_string() + &res.crt_filters.lines_per_pixel.to_string()).into())?;
        dispatch_event_with("app-event.change_lines_per_pixel", &(res.crt_filters.lines_per_pixel as i32).into())?;
    }
    Ok(())
}


fn update_pixel_pulse(dt: f32, res: &mut Resources, input: &Input) -> WasmResult<()> {
    if input.showing_pixels_pulse.is_just_pressed() {
        res.crt_filters.showing_pixels_pulse = !res.crt_filters.showing_pixels_pulse;
        dispatch_event_with("app-event.top_message", &(if res.crt_filters.showing_pixels_pulse {"Screen wave ON."} else {"Screen wave OFF."}).into())?;
        dispatch_event_with("app-event.showing_pixels_pulse", &res.crt_filters.showing_pixels_pulse.into())?;
    }

    if res.crt_filters.showing_pixels_pulse {
        res.crt_filters.pixels_pulse += dt * 0.3;
    } else {
        res.crt_filters.pixels_pulse = 0.0;
    }
    Ok(())
}

fn update_crt_filters(dt: f32, res: &mut Resources, input: &Input) -> WasmResult<()> {

    if input.reset_filters {
        res.crt_filters = CrtFilters::new(PIXEL_MANIPULATION_BASE_SPEED);
        res.crt_filters.cur_pixel_width = res.initial_parameters.initial_pixel_width;
        change_frontend_input_values(res)?;
        dispatch_event_with("app-event.top_message", &"All filter options have been reset.".into())?;
        return Ok(());
    }

    if !input.input_focused && input.next_layering_kind.is_just_pressed() {
        res.crt_filters.layering_kind = ((res.crt_filters.layering_kind as i32 + 1) % RenderLayers::LENGTH as i32).into();
        let mut message: &'static str;
        match res.crt_filters.layering_kind {
            RenderLayers::ShadowOnly => {
                res.crt_filters.showing_diffuse_foreground = true;
                res.crt_filters.showing_solid_background = false;
                message = "Shadow only";
            },
            RenderLayers::SolidOnly => {
                res.crt_filters.showing_diffuse_foreground = false;
                res.crt_filters.showing_solid_background = true;
                res.crt_filters.solid_color_weight = 1.0;
                message = "Solid only";
            },
            RenderLayers::ShadowWithSolidBackground75 => {
                res.crt_filters.showing_diffuse_foreground = true;
                res.crt_filters.showing_solid_background = true;
                res.crt_filters.solid_color_weight = 0.75;
                message = "Shadow with 75% Solid background";
            },
            RenderLayers::ShadowWithSolidBackground50 => {
                res.crt_filters.showing_diffuse_foreground = true;
                res.crt_filters.showing_solid_background = true;
                res.crt_filters.solid_color_weight = 0.5;
                message = "Shadow with 50% Solid background";
            },
            RenderLayers::ShadowWithSolidBackground25 => {
                res.crt_filters.showing_diffuse_foreground = true;
                res.crt_filters.showing_solid_background = true;
                res.crt_filters.solid_color_weight = 0.25;
                message = "Shadow with 25% Solid background";
            },
            RenderLayers::LENGTH => unreachable!(),
        };
        dispatch_event_with("app-event.top_message", &format!("Layering kind '{}' selected.", message).into())?;
    }

    if input.next_color_representation_kind.is_just_pressed() {
        res.crt_filters.color_channels = match res.crt_filters.color_channels {
            ColorChannels::Combined => ColorChannels::Overlapping,
            ColorChannels::Overlapping => ColorChannels::SplitHorizontal,
            ColorChannels::SplitHorizontal => ColorChannels::SplitVertical,
            ColorChannels::SplitVertical => ColorChannels::Combined,
        };
        let message = match res.crt_filters.color_channels {
            ColorChannels::Combined => "combined",
            ColorChannels::Overlapping => "horizontal overlapping",
            ColorChannels::SplitHorizontal => "horizontal split",
            ColorChannels::SplitVertical => "vertical split",
        };
        dispatch_event_with("app-event.top_message", &("Pixel color representation: ".to_string() + message + ".").into())?;
    }

    if input.next_pixel_geometry_kind.is_just_released() {
        res.crt_filters.pixels_geometry_kind = match res.crt_filters.pixels_geometry_kind {
            PixelsGeometryKind::Squares => PixelsGeometryKind::Cubes,
            PixelsGeometryKind::Cubes => PixelsGeometryKind::Squares
        };
        let message = match res.crt_filters.pixels_geometry_kind {
            PixelsGeometryKind::Squares => "squares",
            PixelsGeometryKind::Cubes => "cubes"
        };
        dispatch_event_with("app-event.top_message", &("Showing pixels as ".to_string() + message + ".").into())?;
        dispatch_event_with("app-event.showing_pixels_as", &message.into())?;
    }

    if !input.input_focused && input.toggle_pixels_shadow_kind.is_just_released() {
        res.crt_filters.pixel_shadow_kind += 1;
        if res.crt_filters.pixel_shadow_kind >= res.pixels_render.shadows_len() {
            res.crt_filters.pixel_shadow_kind = 0;
        }
        dispatch_event_with("app-event.top_message", &("Showing next pixel shadow: ".to_string() + &res.crt_filters.pixel_shadow_kind.to_string() + ".").into())?;
    }

    let pixel_velocity = dt * res.crt_filters.change_speed;
    change_pixel_sizes(&input, input.increase_pixel_scale_x, input.decrease_pixel_scale_x, &mut res.crt_filters.cur_pixel_scale_x, pixel_velocity * 0.00125, "app-event.change_pixel_vertical_gap", "event_kind:pixel_vertical_gap")?;
    change_pixel_sizes(&input, input.increase_pixel_scale_y, input.decrease_pixel_scale_y, &mut res.crt_filters.cur_pixel_scale_y, pixel_velocity * 0.00125, "app-event.change_pixel_horizontal_gap", "event_kind:pixel_horizontal_gap")?;
    change_pixel_sizes(&input, input.increase_pixel_gap && !input.shift, input.decrease_pixel_gap && !input.shift, &mut res.crt_filters.cur_pixel_width, pixel_velocity * 0.005, "app-event.change_pixel_width", "event_kind:pixel_width")?;
    change_pixel_sizes(&input, input.increase_pixel_gap && input.shift, input.decrease_pixel_gap && input.shift, &mut res.crt_filters.cur_pixel_gap, pixel_velocity * 0.005, "app-event.change_pixel_spread", "event_kind:pixel_spread")?;

    fn change_pixel_sizes(input: &Input, increase: bool, decrease: bool, cur_size: &mut f32, velocity: f32, event_id: &str, event_kind: &str) -> WasmResult<()> {
        let before_size = *cur_size;
        if increase {
            *cur_size += velocity;
        }
        if decrease {
            *cur_size -= velocity;
        }
        if input.custom_event.kind.as_ref() as &str == event_kind {
            *cur_size = input.custom_event.value.as_f64().ok_or("it should be a number")? as f32;
        }
        if *cur_size != before_size {
            if *cur_size < 0.0 {
                *cur_size = 0.0;
                dispatch_event_with("app-event.top_message", &"Minimum value is 0.0".into())?;
            }
            let size = *cur_size;
            dispatch_event_with(event_id, &size.into())?;
        }
        Ok(())
    }
    Ok(())
}

fn update_speeds(res: &mut Resources, input: &Input) -> WasmResult<()> {
    if input.alt {
        //change_speed(&input, &mut res.camera.turning_speed, TURNING_BASE_SPEED, "Turning camera speed: ")?;
    } else if input.shift {
        change_speed(&input, &mut res.crt_filters.change_speed, PIXEL_MANIPULATION_BASE_SPEED, "Pixel manipulation speed: ", "app-event.change_pixel_speed")?;
    } else {
        change_speed(&input, &mut res.camera.turning_speed, TURNING_BASE_SPEED, "Turning camera speed: ", "app-event.change_turning_speed")?;
        change_speed(&input, &mut res.camera.movement_speed, res.initial_parameters.initial_movement_speed, "Translation camera speed: ", "app-event.change_movement_speed")?;
    }

    fn change_speed(input: &Input, cur_speed: &mut f32, base_speed: f32, top_message: &str, event_id: &str) -> WasmResult<()> {
        let before_speed = *cur_speed;
        if input.speed_up.is_just_pressed() && *cur_speed < 10000.0 { *cur_speed *= 2.0; }
        if input.speed_down.is_just_pressed() && *cur_speed > 0.01 { *cur_speed /= 2.0; }
        if *cur_speed != before_speed {
            let speed = (*cur_speed / base_speed * 1000.0).round() / 1000.0;
            let message = top_message.to_string() + &speed.to_string() + &"x".to_string();
            dispatch_event_with("app-event.top_message", &message.into())?;
            dispatch_event_with(event_id, &speed.into())?;
        }
        Ok(())
    }

    if input.reset_speeds {
        res.camera.turning_speed = TURNING_BASE_SPEED;
        res.camera.movement_speed = res.initial_parameters.initial_movement_speed;
        res.crt_filters.change_speed = PIXEL_MANIPULATION_BASE_SPEED;
        dispatch_event_with("app-event.top_message", &"All speeds have been reset.".into())?;
        change_frontend_input_values(res)?;
    }
    Ok(())
}

fn update_camera(dt: f32, res: &mut Resources, input: &Input) -> WasmResult<()> {
    if input.walk_left { res.camera.advance(CameraDirection::Left, dt); }
    if input.walk_right { res.camera.advance(CameraDirection::Right, dt); }
    if input.walk_up { res.camera.advance(CameraDirection::Up, dt); }
    if input.walk_down { res.camera.advance(CameraDirection::Down, dt); }
    if input.walk_forward { res.camera.advance(CameraDirection::Forward, dt); }
    if input.walk_backward { res.camera.advance(CameraDirection::Backward, dt); }

    if input.turn_left { res.camera.turn(CameraDirection::Left, dt); }
    if input.turn_right { res.camera.turn(CameraDirection::Right, dt); }
    if input.turn_up { res.camera.turn(CameraDirection::Up, dt); }
    if input.turn_down { res.camera.turn(CameraDirection::Down, dt); }

    if input.input_focused == false { // Because it's hotkey '+' '-', writitng on fields can get messy.
        if input.rotate_left { res.camera.rotate(CameraDirection::Left, dt); }
        if input.rotate_right { res.camera.rotate(CameraDirection::Right, dt); }
    }

    if input.mouse_click.is_just_pressed() {
        dispatch_event("app-event.request_pointer_lock")?;
    } else if input.mouse_click.is_activated() {
        res.camera.drag(input.mouse_position_x, input.mouse_position_y);
    } else if input.mouse_click.is_just_released() {
        dispatch_event("app-event.exit_pointer_lock")?;
    }

    if input.increase_camera_zoom {
        res.camera.change_zoom(dt * -100.0)?;
    } else if input.decrease_camera_zoom {
        res.camera.change_zoom(dt * 100.0)?;
    } else if input.mouse_scroll_y != 0.0 {
        res.camera.change_zoom(input.mouse_scroll_y)?;
    }

    // @Refactor too much code for too little stuff done in this match.
    match input.custom_event.kind.as_ref() {

        "event_kind:camera_zoom" => {
            res.camera.zoom = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
        },

        "event_kind:camera_pos_x" => {
            let mut position = res.camera.get_position();
            position.x = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_position(position);
        },
        "event_kind:camera_pos_y" => {
            let mut position = res.camera.get_position();
            position.y = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_position(position);
        },
        "event_kind:camera_pos_z" => {
            let mut position = res.camera.get_position();
            position.z = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_position(position);
        },

        "event_kind:camera_axis_up_x" => {
            let mut axis_up = res.camera.get_axis_up();
            axis_up.x = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_axis_up(axis_up);
        },
        "event_kind:camera_axis_up_y" => {
            let mut axis_up = res.camera.get_axis_up();
            axis_up.y = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_axis_up(axis_up);
        },
        "event_kind:camera_axis_up_z" => {
            let mut axis_up = res.camera.get_axis_up();
            axis_up.z = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_axis_up(axis_up);
        },

        "event_kind:camera_direction_x" => {
            let mut direction = res.camera.get_direction();
            direction.x = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_direction(direction);
        },
        "event_kind:camera_direction_y" => {
            let mut direction = res.camera.get_direction();
            direction.y = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_direction(direction);
        },
        "event_kind:camera_direction_z" => {
            let mut direction = res.camera.get_direction();
            direction.z = input.custom_event.value.as_f64().ok_or("Wrong number")? as f32;
            res.camera.set_direction(direction);
        },

        _ => {}
    }

    if input.reset_position {
        res.camera.set_position(glm::vec3(0.0, 0.0, res.initial_parameters.initial_position_z));
        res.camera.set_direction(glm::vec3(0.0, 0.0, -1.0));
        res.camera.set_axis_up(glm::vec3(0.0, 1.0, 0.0));
        res.camera.zoom = 45.0;
        dispatch_event_with("app-event.change_camera_zoom", &res.camera.zoom.into())?;
        dispatch_event_with("app-event.top_message", &"The camera have been reset.".into())?;
    }

    res.camera.update_view()
}

pub fn draw(gl: &WebGl2RenderingContext, res: &Resources) -> WasmResult<()> {

    gl.enable(WebGl2RenderingContext::DEPTH_TEST);
    gl.clear_color(0.0, 0.0, 0.0, 0.0);

    //gl.enable(WebGl2RenderingContext::BLEND);
    //gl.blend_func(WebGl2RenderingContext::SRC_ALPHA, WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA);

    if res.animation.needs_buffer_data_load {
        res.pixels_render.apply_colors(gl, &res.animation.steps[res.animation.current_frame]);
    }

    let mut buffer_stack = res.texture_buffer_stack.borrow_mut();
    buffer_stack.push(gl)?;
    buffer_stack.push(gl)?;
    buffer_stack.bind_current(gl)?;
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT|WebGl2RenderingContext::DEPTH_BUFFER_BIT);

    if res.crt_filters.showing_diffuse_foreground {
        let mut extra_light = get_3_f32color_from_int(res.crt_filters.brightness_color);
        for light in extra_light.iter_mut() {
            *light *= res.crt_filters.extra_bright;
        }
        let vertical_lines_ratio = res.crt_filters.lines_per_pixel;
        for j in 0..vertical_lines_ratio {
            let color_splits = match res.crt_filters.color_channels {ColorChannels::Combined => 1, _ => 3};
            for i in 0..color_splits {
                let mut light_color = get_3_f32color_from_int(res.crt_filters.light_color);
                let pixel_offset = &mut [0.0, 0.0, 0.0];
                let pixel_scale = &mut [
                    (res.crt_filters.cur_pixel_scale_x + 1.0) / res.crt_filters.cur_pixel_width,
                    res.crt_filters.cur_pixel_scale_y + 1.0,
                    (res.crt_filters.cur_pixel_scale_x + res.crt_filters.cur_pixel_scale_x) * 0.5 + 1.0,
                ];
                match res.crt_filters.color_channels {
                    ColorChannels::Combined => {},
                    _ => {
                        light_color[(i + 0) % 3] *= 1.0;
                        light_color[(i + 1) % 3] = 0.0;
                        light_color[(i + 2) % 3] = 0.0;
                        match res.crt_filters.color_channels {
                            ColorChannels::SplitHorizontal => {
                                pixel_offset[0] = (i as f32 - 1.0) * (1.0 / 3.0) * res.crt_filters.cur_pixel_width / (res.crt_filters.cur_pixel_scale_x + 1.0);
                                pixel_scale[0] *= color_splits as f32;
                            },
                            ColorChannels::Overlapping => {
                                pixel_offset[0] = (i as f32 - 1.0) * (1.0 / 3.0) * res.crt_filters.cur_pixel_width / (res.crt_filters.cur_pixel_scale_x + 1.0);
                                pixel_scale[0] *= 1.5;
                            },
                            ColorChannels::SplitVertical => {
                                pixel_offset[1] = (i as f32 - 1.0) * (1.0 / 3.0) * (1.0 - res.crt_filters.cur_pixel_scale_y);
                                pixel_scale[1] *= color_splits as f32;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                if vertical_lines_ratio > 1 {
                    pixel_offset[0] /= vertical_lines_ratio as f32;
                    pixel_offset[0] += (j as f32 / vertical_lines_ratio as f32 - calc_stupid_not_extrapoled_function(vertical_lines_ratio)) * res.crt_filters.cur_pixel_width / (res.crt_filters.cur_pixel_scale_x + 1.0);
                    pixel_scale[0] *= vertical_lines_ratio as f32;
                }
                if let ColorChannels::Overlapping = res.crt_filters.color_channels {
                    buffer_stack.push(gl)?;
                    buffer_stack.bind_current(gl)?;
                    if j == 0 {
                        gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT|WebGl2RenderingContext::DEPTH_BUFFER_BIT);
                    }
                }
                //gl.blend_func(WebGl2RenderingContext::SRC_ALPHA, WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA);
                res.pixels_render.render(gl, PixelsUniform {
                    shadow_kind: res.crt_filters.pixel_shadow_kind,
                    geometry_kind: res.crt_filters.pixels_geometry_kind,
                    view: res.camera.get_view().as_mut_slice(),
                    projection: res.camera.get_projection(
                        res.animation.viewport_width as f32,
                        res.animation.viewport_height as f32,
                    ).as_mut_slice(),
                    ambient_strength: match res.crt_filters.pixels_geometry_kind { PixelsGeometryKind::Squares => 1.0   , PixelsGeometryKind::Cubes => 0.5},
                    contrast_factor: res.crt_filters.extra_contrast,
                    light_color: &mut light_color,
                    extra_light: &mut extra_light,
                    light_pos: res.camera.get_position().as_mut_slice(),
                    pixel_gap: &mut [
                        (1.0 + res.crt_filters.cur_pixel_gap) * res.crt_filters.cur_pixel_width,
                        1.0 + res.crt_filters.cur_pixel_gap,
                    ],
                    pixel_scale,
                    pixel_pulse: res.crt_filters.pixels_pulse,
                    pixel_offset,
                    height_modifier_factor: 1.0,
                });
            }
            if let ColorChannels::Overlapping = res.crt_filters.color_channels {
                buffer_stack.pop()?;
                buffer_stack.pop()?;
                buffer_stack.pop()?;
            }
        }

        if let ColorChannels::Overlapping = res.crt_filters.color_channels {
            buffer_stack.bind_current(gl)?;
            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);
            gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, buffer_stack.get_nth(1)?.texture());
            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 1);
            gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, buffer_stack.get_nth(2)?.texture());
            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 2);
            gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, buffer_stack.get_nth(3)?.texture());

            res.rgb_render.render(gl);

            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);
        }
    }

    buffer_stack.push(gl)?;
    buffer_stack.bind_current(gl)?;
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

    if res.crt_filters.showing_solid_background {
        res.pixels_render.render(gl, PixelsUniform {
            shadow_kind: 0,
            geometry_kind: res.crt_filters.pixels_geometry_kind,
            view: res.camera.get_view().as_mut_slice(),
            projection: res.camera.get_projection(
                res.animation.viewport_width as f32,
                res.animation.viewport_height as f32,
            ).as_mut_slice(),
            ambient_strength: match res.crt_filters.pixels_geometry_kind { PixelsGeometryKind::Squares => 1.0, PixelsGeometryKind::Cubes => 0.5},
            contrast_factor: res.crt_filters.extra_contrast,
            light_color: &mut [res.crt_filters.solid_color_weight, res.crt_filters.solid_color_weight, res.crt_filters.solid_color_weight],
            extra_light: &mut [0.0, 0.0, 0.0],
            light_pos: res.camera.get_position().as_mut_slice(),
            pixel_gap: &mut [
                (1.0 + res.crt_filters.cur_pixel_gap) * res.crt_filters.cur_pixel_width,
                1.0 + res.crt_filters.cur_pixel_gap,
            ],
            pixel_scale: &mut [
                (res.crt_filters.cur_pixel_scale_x + 1.0) / res.crt_filters.cur_pixel_width,
                res.crt_filters.cur_pixel_scale_y + 1.0,
                (res.crt_filters.cur_pixel_scale_x + res.crt_filters.cur_pixel_scale_x) * 0.5 + 1.0,
            ],
            pixel_pulse: res.crt_filters.pixels_pulse,
            pixel_offset: &mut [0.0, 0.0, 0.0],
            height_modifier_factor: 0.0,
        });
    }
    buffer_stack.pop()?;
    buffer_stack.pop()?;
    buffer_stack.bind_current(gl)?;
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT|WebGl2RenderingContext::DEPTH_BUFFER_BIT);

    gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, buffer_stack.get_nth(1)?.texture());
    gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 1);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, buffer_stack.get_nth(2)?.texture());
    res.background_render.render(gl);
    gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);

    if res.crt_filters.blur_passes > 0 {
        res.blur_render.render(&gl, res.crt_filters.blur_passes, &mut buffer_stack)?;
    }

    if res.launch_screenshot {
        let multiplier : i32 = res.internal_resolution_multiplier;
        let width = res.animation.viewport_width as i32 * multiplier;
        let height = res.animation.viewport_height as i32 * multiplier;
        let pixels = js_sys::Uint8Array::new(&(width * height * 4).into());
        gl.read_pixels_with_opt_array_buffer_view(0, 0, width, height, WebGl2RenderingContext::RGBA, WebGl2RenderingContext::UNSIGNED_BYTE, Some(&pixels))?;
        let array = js_sys::Array::new();
        array.push(&pixels);
        array.push(&multiplier.into());
        dispatch_event_with("app-event.screenshot", &array)?;
    }

    buffer_stack.pop()?;
    buffer_stack.assert_no_stack()?;
    //gl.blend_func(WebGl2RenderingContext::SRC_ALPHA, WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA);

    gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
    gl.viewport(0, 0, res.animation.viewport_width as i32, res.animation.viewport_height as i32);


    res.internal_resolution_render.render(gl, buffer_stack.get_nth(1)?.texture());

    check_error(&gl, line!())?;

    Ok(())
}

pub fn check_error(gl: &WebGl2RenderingContext, line: u32) -> WasmResult<()> {
    let error = gl.get_error();
    if error != WebGl2RenderingContext::NO_ERROR {
        return Err(WasmError::Str(error.to_string() + " on line: " + &line.to_string()));
    }
    Ok(())
}

pub fn get_3_f32color_from_int(color: i32) -> [f32; 3] {[
    (color >> 16) as f32 / 255.0,
    ((color >> 8) & 0xFF) as f32 / 255.0,
    (color & 0xFF) as f32 / 255.0,
]}

fn calc_stupid_not_extrapoled_function(y: usize) -> f32 {
    match y {
        1 => (0.0),
        2 => (0.25),
        3 => (1.0 / 3.0),
        4 => (0.375),
        5 => (0.4),
        6 => (0.4 + 0.1 / 6.0),
        7 => (0.4 + 0.1 / 6.0 + 0.1 / 8.4),
        8 => (0.4 + 0.1 / 6.0 + 0.1 / 8.4 + 0.00892575),
        9 => (0.4 + 0.1 / 6.0 + 0.1 / 8.4 + 0.00892575 + 0.006945),
        _ => (0.45) // originalmente: 0.4 + 0.1 / 6.0 + 0.1 / 8.4 + 0.00892575 + 0.006945 + 0.0055555555
    }
/*
Let's consider this was a function where we find the following points:
f(1) = 0
0.25
f(2) = 0.25
0.08333333333 | 0.33333
f(3) = 0.33333333333
0.0416666666 | 0.5
f(4) = 0.375
0.025 | 0.6
f(5) = 0.4
0.0166666666666 | 0.6666666666
f(6) = 0.41666666666
0.01190476190475190476190 | 0.71428571424028571
f(7) = 0.42857142857
0.00892575 | 0.749763
f(8) = 0.43749717857142857142857142857143
0.006945 | 0.77808587513
f(9) = 0.444442178571428571428
0.00555555555555555555555 | 0.79999
f(10) = 0.45

It looks like this function is growing less than a logarithmic one
*/
}

#[cfg(test)]
mod tests { mod get_3_f32color_from_int { mod gives_good {
    use super::super::super::*;

    macro_rules! get_3_f32color_from_int_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, expected) = $value;
                assert_eq!(expected, get_3_f32color_from_int(input));
            }
        )*
        }
    }

    get_3_f32color_from_int_tests! {
        white: (0x00FF_FFFF, [1.0, 1.0, 1.0]),
        black: (0x0000_0000, [0.0, 0.0, 0.0]),
        red: (0x00FF_0000, [1.0, 0.0, 0.0]),
        green: (0x0000_FF00, [0.0, 1.0, 0.0]),
        blue: (0x0000_00FF, [0.0, 0.0, 1.0]),
        yellow: (0x00eb_f114, [0.92156863, 0.94509804, 0.078431375]),
    }
} } }