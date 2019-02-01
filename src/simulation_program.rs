use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use web_sys::{
    console,
    Window,
    WebGl2RenderingContext,
};
use std::rc::Rc;
use super::glm;

use wasm_error::{WasmResult, WasmError};
use camera::{CameraDirection, Camera};
use dispatch_event::{dispatch_event, dispatch_event_with};
use web_utils::{now, window};
use pixels_render::{PixelsRender, PixelsRenderKind, PixelsUniform};
use blur_render::BlurRender;
use event_listeners::{set_event_listeners, on_button_action};
use simulation_state::{StateOwner, Resources, Input, AnimationData, Buttons};

const PIXEL_MANIPULATION_BASE_SPEED: f32 = 20.0;
const TURNING_BASE_SPEED: f32 = 3.0;
const MOVEMENT_BASE_SPEED: f32 = 10.0;
const MOVEMENT_SPEED_FACTOR: f32 = 50.0;

pub fn program(gl: JsValue, animation: AnimationData) -> WasmResult<()> {
    let gl = gl.dyn_into::<WebGl2RenderingContext>()?;
    gl.viewport(0, 0, animation.viewport_width as i32, animation.viewport_height as i32);
    gl.enable(WebGl2RenderingContext::DEPTH_TEST);
    gl.enable(WebGl2RenderingContext::BLEND);
    let owned_state = StateOwner::new_rc(load_resources(&gl, animation)?, Input::new()?);
    let frame_closure: Closure<FnMut(JsValue)> = {
        let owned_state = Rc::clone(&owned_state);
        let window = window()?;
        Closure::wrap(Box::new(move |_| {
            if let Err(e) = program_iteration(&owned_state, &gl, &window) {
                console::error_2(&"An unexpected error happened during program_iteration.".into(), &e.to_js());
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
        console::log_1(&"User closed the simulation.".into());
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
    let now = now()?;
    let res = Resources {
        initial_position_z,
        initial_pixel_width: animation.scale_x,
        pixels_render: PixelsRender::new(&gl, animation.image_width as usize, animation.image_height as usize)?,
        blur_render: BlurRender::new(&gl, animation.viewport_width as i32, animation.viewport_height as i32)?,
        light_color: 0x00FF_FFFF,
        brightness_color: 0x00FF_FFFF,
        extra_bright: 0.0,
        frame_count: 0,
        translation_base_speed: camera.movement_speed,
        last_time: now,
        last_second: now,
        blur_passes: 0,
        pixel_manipulation_speed: PIXEL_MANIPULATION_BASE_SPEED,
        showing_split_colors: false,
        cur_pixel_scale_x: 0.0,
        cur_pixel_scale_y: 0.0,
        cur_pixel_gap: 0.0,
        pixels_render_kind: PixelsRenderKind::Squares,
        pixels_pulse: 0.0,
        showing_pixels_pulse: false,
        animation,
        camera,
        camera_zoom: 45.0,
        buttons: Buttons::new()
    };
    change_frontend_input_values(&res)?;
    Ok(res)
}

fn change_frontend_input_values(res: &Resources) -> WasmResult<()> {
    dispatch_event_with("app-event.change_pixel_horizontal_gap", &res.cur_pixel_scale_x.into())?;
    dispatch_event_with("app-event.change_pixel_vertical_gap", &res.cur_pixel_scale_y.into())?;
    dispatch_event_with("app-event.change_pixel_width", &res.animation.scale_x.into())?;
    dispatch_event_with("app-event.change_pixel_spread", &res.cur_pixel_gap.into())?;
    dispatch_event_with("app-event.change_pixel_brightness", &res.extra_bright.into())?;
    dispatch_event_with("app-event.change_light_color", &res.light_color.into())?;
    dispatch_event_with("app-event.change_brightness_color", &res.brightness_color.into())?;
    Ok(())
}

fn calculate_far_away_position(animation: &AnimationData) -> f32 {
    let width = animation.background_width as f32;
    let height = animation.background_height as f32;
    let viewport_width_scaled = (animation.viewport_width as f32 / animation.scale_x) as u32;
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
    0.5 + (resolution as f32 / bound_ratio) * if is_height_bounded {1.2076} else {0.68 * animation.scale_x}
}

fn pre_process_input(input: &mut Input, resources: &Resources) -> WasmResult<()> {
    input.now = now().unwrap_or(resources.last_time);
    match input.custom_event.kind.as_ref() {
        "button_down" => {
            let button = input.custom_event.value.as_string().ok_or("invalid-botton-down")?;
            on_button_action(input, button.as_ref(), true);
        },
        "button_up" => {
            let button = input.custom_event.value.as_string().ok_or("invalid-botton-up")?;
            on_button_action(input, button.as_ref(), false);
        },
        _ => {}
    }
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

    res.buttons.esc.track(input.esc);
    if res.buttons.esc.is_just_pressed() {
        dispatch_event("app-event.exiting_session")?;
        return Ok(false);
    }

    res.buttons.space.track(input.space);
    if res.buttons.space.is_just_pressed() {
        dispatch_event("app-event.toggle_info_panel")?;
    }

    update_pixel_pulse(dt, res, input)?;
    update_pixel_characteristics(dt, res, input)?;
    update_speeds(res, input)?;
    update_view_and_perspective(dt, res, input)?;

    Ok(true)
}

fn update_timers_and_dt(res: &mut Resources, input: &Input) -> WasmResult<f32> {
    let dt: f32 = ((input.now - res.last_time) / 1000.0) as f32;
    let ellapsed = input.now - res.last_second;
    res.last_time = input.now;

    if ellapsed >= 1_000.0 {
        let fps = res.frame_count as f32;
        dispatch_event_with("app-event.fps", &fps.into())?;
        res.last_second = input.now;
        res.frame_count = 0;
    } else {
        res.frame_count += 1;
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
        res.extra_bright += 0.01 * dt * res.pixel_manipulation_speed;
    }
    if input.decrease_bright {
        res.extra_bright -= 0.01 * dt * res.pixel_manipulation_speed;
    }
    if input.increase_bright || input.decrease_bright {
        if res.extra_bright < -1.0 {
            res.extra_bright = -1.0;
        } else if res.extra_bright > 1.0 {
            res.extra_bright = 1.0;
        } else {
            dispatch_event_with("app-event.change_pixel_brightness", &res.extra_bright.into())?;
        }
    }
    if input.custom_event.kind.as_ref() as &str == "event_kind:pixel_brightness" {
        res.extra_bright = input.custom_event.value.as_f64().ok_or("it should be a number")? as f32;
    }

    let color_variable = match input.custom_event.kind.as_ref() {
        "event_kind:light_color" => &mut res.light_color,
        "event_kind:brightness_color" => &mut res.brightness_color,
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
    let last_blur_passes = res.blur_passes;
    res.buttons.increase_blur.track(input.increase_blur);
    res.buttons.decrease_blur.track(input.decrease_blur);
    if res.buttons.increase_blur.is_just_pressed() {
        res.blur_passes += 1;
    }
    if res.buttons.decrease_blur.is_just_pressed() && res.blur_passes > 0 {
        res.blur_passes -= 1;
    }
    if input.custom_event.kind.as_ref() as &str == "event_kind:blur_level" {
        res.blur_passes = input.custom_event.value.as_f64().ok_or("it should be a number")? as usize;
    }

    if last_blur_passes != res.blur_passes {
        dispatch_event_with("app-event.top_message", &("Blur level: ".to_string() + &res.blur_passes.to_string()).into())?;
        dispatch_event_with("app-event.change_blur_level", &(res.blur_passes as i32).into())?;
    }
    Ok(())
}

fn update_pixel_pulse(dt: f32, res: &mut Resources, input: &Input) -> WasmResult<()> {
    res.buttons.showing_pixels_pulse.track(input.showing_pixels_pulse);
    if res.buttons.showing_pixels_pulse.is_just_pressed() {
        res.showing_pixels_pulse = !res.showing_pixels_pulse;
        dispatch_event_with("app-event.top_message", &(if res.showing_pixels_pulse {"Screen wave ON."} else {"Screen wave OFF."}).into())?;
        dispatch_event_with("app-event.showing_pixels_pulse", &res.showing_pixels_pulse.into())?;
    }

    if res.showing_pixels_pulse {
        res.pixels_pulse += dt * 0.3;
    } else {
        res.pixels_pulse = 0.0;
    }
    Ok(())
}

fn update_pixel_characteristics(dt: f32, res: &mut Resources, input: &Input) -> WasmResult<()> {

    if input.reset_filters {
        res.animation.scale_x = res.initial_pixel_width;
        res.light_color = 0x00FF_FFFF;
        res.brightness_color = 0x00FF_FFFF;
        res.extra_bright = 0.0;
        res.blur_passes = 0;
        res.showing_split_colors = false;
        res.cur_pixel_scale_x = 0.0;
        res.cur_pixel_scale_y = 0.0;
        res.cur_pixel_gap = 0.0;
        res.pixels_render_kind = PixelsRenderKind::Squares;
        res.pixels_pulse = 0.0;
        res.showing_pixels_pulse = false;
        change_frontend_input_values(res)?;
        return Ok(());
    }

    res.buttons.toggle_split_colors.track(input.toggle_split_colors);
    if res.buttons.toggle_split_colors.is_just_pressed() {
        res.showing_split_colors = !res.showing_split_colors;
        let message = if res.showing_split_colors { "Individually" } else { "Combined" };
        dispatch_event_with("app-event.top_message", &("Showing color channels ".to_string() + message + ".").into())?;
    }

    res.buttons.toggle_pixels_render_kind.track(input.toggle_pixels_render_kind);
    if res.buttons.toggle_pixels_render_kind.is_just_released() {
        res.pixels_render_kind = match res.pixels_render_kind {
            PixelsRenderKind::Squares => PixelsRenderKind::Cubes,
            PixelsRenderKind::Cubes => PixelsRenderKind::Squares
        };
        let message = match res.pixels_render_kind {
            PixelsRenderKind::Squares => "squares",
            PixelsRenderKind::Cubes => "cubes"
        };
        dispatch_event_with("app-event.top_message", &("Showing pixels as ".to_string() + message + ".").into())?;
        dispatch_event_with("app-event.showing_pixels_as", &message.into())?;
    }

    let pixel_velocity = dt * res.pixel_manipulation_speed;
    change_pixel_sizes(&input, input.increase_pixel_scale_x, input.decrease_pixel_scale_x, &mut res.cur_pixel_scale_x, pixel_velocity * 0.00125, "app-event.change_pixel_vertical_gap", "event_kind:pixel_vertical_gap")?;
    change_pixel_sizes(&input, input.increase_pixel_scale_y, input.decrease_pixel_scale_y, &mut res.cur_pixel_scale_y, pixel_velocity * 0.00125, "app-event.change_pixel_horizontal_gap", "event_kind:pixel_horizontal_gap")?;
    change_pixel_sizes(&input, input.increase_pixel_gap && !input.shift, input.decrease_pixel_gap && !input.shift, &mut res.animation.scale_x, pixel_velocity * 0.005, "app-event.change_pixel_width", "event_kind:pixel_width")?;
    change_pixel_sizes(&input, input.increase_pixel_gap && input.shift, input.decrease_pixel_gap && input.shift, &mut res.cur_pixel_gap, pixel_velocity * 0.005, "app-event.change_pixel_spread", "event_kind:pixel_spread")?;

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
            }
            let size = *cur_size;
            dispatch_event_with(event_id, &size.into())?;
        }
        Ok(())
    }
    Ok(())
}

fn update_speeds(res: &mut Resources, input: &Input) -> WasmResult<()> {
    res.buttons.speed_up.track(input.speed_up);
    res.buttons.speed_down.track(input.speed_down);
    if input.alt {
        change_speed(&res.buttons, &mut res.camera.turning_speed, TURNING_BASE_SPEED, "app-event.pixelturning_speed_manipulation_speed", "Turning camera speed: ")?;
    } else if input.shift {
        change_speed(&res.buttons, &mut res.pixel_manipulation_speed, PIXEL_MANIPULATION_BASE_SPEED, "app-event.pixel_manipulation_speed", "Pixel manipulation speed: ")?;
    } else {
        change_speed(&res.buttons, &mut res.camera.movement_speed, res.translation_base_speed, "app-event.translation_speed", "Translation camera speed: ")?;
    }

    fn change_speed(buttons: &Buttons, cur_speed: &mut f32, base_speed: f32, event_id: &str, top_message: &str) -> WasmResult<()> {
        if buttons.speed_up.is_just_pressed() && *cur_speed < 10000.0 { *cur_speed *= 2.0; }
        if buttons.speed_down.is_just_pressed() && *cur_speed > 0.01 { *cur_speed /= 2.0; }
        if buttons.speed_up.is_just_pressed() || buttons.speed_down.is_just_pressed() {
            let speed = (*cur_speed / base_speed * 1000.0).round() / 1000.0;
            let message = top_message.to_string() + &speed.to_string() + &"x".to_string();
            dispatch_event_with("app-event.top_message", &message.into())?;
            dispatch_event_with(event_id, &speed.into())?;
        }
        Ok(())
    }

    if input.reset_speeds {
        res.camera.turning_speed = TURNING_BASE_SPEED;
        res.camera.movement_speed = res.translation_base_speed;
        res.pixel_manipulation_speed = PIXEL_MANIPULATION_BASE_SPEED;
        dispatch_event_with("app-event.top_message", &"All speeds have been reset.".into())?;
        dispatch_event("app-event.speed_reset")?;
    }
    Ok(())
}

fn update_view_and_perspective(dt: f32, res: &mut Resources, input: &Input) -> WasmResult<()> {
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

    if input.rotate_left { res.camera.rotate(CameraDirection::Left, dt); }
    if input.rotate_right { res.camera.rotate(CameraDirection::Right, dt); }

    res.buttons.mouse_click.track(input.mouse_left_click);
    if res.buttons.mouse_click.is_just_pressed() {
        dispatch_event("app-event.request_pointer_lock")?;
    } else if res.buttons.mouse_click.is_activated() {
        res.camera.drag(input.mouse_position_x, input.mouse_position_y);
    } else if res.buttons.mouse_click.is_just_released() {
        dispatch_event("app-event.exit_pointer_lock")?;
    }

    if input.mouse_scroll_y != 0.0 {
        if res.camera_zoom >= 1.0 && res.camera_zoom <= 45.0 {
            res.camera_zoom -= input.mouse_scroll_y * 0.1;
        }
        if res.camera_zoom <= 1.0 {
            res.camera_zoom = 1.0;
        }
        if res.camera_zoom >= 45.0 {
            res.camera_zoom = 45.0;
        }
    }

    // @Refactor too much code for too little stuff done in this match.
    match input.custom_event.kind.as_ref() {
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
        res.camera.set_position(glm::vec3(0.0, 0.0, res.initial_position_z));
        res.camera.set_direction(glm::vec3(0.0, 0.0, -1.0));
        res.camera.set_axis_up(glm::vec3(0.0, 1.0, 0.0));
    }

    res.camera.update_view()
}

pub fn draw(gl: &WebGl2RenderingContext, res: &Resources) -> WasmResult<()> {
    if res.animation.needs_buffer_data_load {
        res.pixels_render.apply_colors(gl, &res.animation.steps[res.animation.current_frame]);
    }

    gl.clear_color(0.05, 0.05, 0.05, 0.0);
    if res.blur_passes > 0 {
        res.blur_render.pre_render(&gl);
    }
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT|WebGl2RenderingContext::DEPTH_BUFFER_BIT);
    gl.blend_func(WebGl2RenderingContext::SRC_ALPHA, WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA);

    let mut extra_light = get_3_f32color_from_int(res.brightness_color);
    for light in extra_light.iter_mut() {
        *light *= res.extra_bright;
    }

    let color_splits = if res.showing_split_colors {3} else {1};
    for i in 0..color_splits {
        let mut light_color = get_3_f32color_from_int(res.light_color);
        let mut pixel_offset = 0.0;
        if res.showing_split_colors {
            light_color[(i + 0) % 3] *= 1.5;
            light_color[(i + 1) % 3] = 0.0;
            light_color[(i + 2) % 3] = 0.0;
            pixel_offset = (i as f32 - 1.0) * res.animation.scale_x * (1.0 / 3.0) + match i % 3 {
                0 => res.cur_pixel_scale_x * (1.0 / 3.0),
                1 => 0.0,
                2 => - res.cur_pixel_scale_x * (1.0 / 3.0),
                _ => unreachable!(),
            };
        }
        res.pixels_render.render(gl, &res.pixels_render_kind, PixelsUniform {
            view: res.camera.get_view().as_mut_slice(),
            projection: glm::perspective::<f32>(
                res.animation.viewport_width as f32 / res.animation.viewport_height as f32,
                radians(res.camera_zoom),
                0.01,
                10000.0
            ).as_mut_slice(),
            ambient_strength: match res.pixels_render_kind { PixelsRenderKind::Squares => 1.0, PixelsRenderKind::Cubes => 0.5},
            light_color: &mut light_color,
            extra_light: &mut extra_light,
            light_pos: res.camera.get_position().as_mut_slice(),
            pixel_gap: &mut [
                (1.0 + res.cur_pixel_gap) * res.animation.scale_x,
                1.0 + res.cur_pixel_gap,
            ],
            pixel_scale: &mut [
                (res.cur_pixel_scale_x + 1.0) / (res.animation.scale_x / color_splits as f32),
                res.cur_pixel_scale_y + 1.0,
                (res.cur_pixel_scale_x + res.cur_pixel_scale_x) * 0.5 + 1.0,
            ],
            pixel_pulse: res.pixels_pulse,
            pixel_offset,
        });
    }

    if res.blur_passes > 0 {
        gl.blend_func(WebGl2RenderingContext::ONE, WebGl2RenderingContext::ZERO);
        res.blur_render.render(&gl, res.blur_passes);
    }

    check_error(&gl, line!())?;

    Ok(())
}

pub fn radians(grad: f32) -> f32 {
    let pi: f32 = glm::pi();
    grad * pi / 180.0
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