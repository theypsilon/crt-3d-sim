use web_sys::WebGl2RenderingContext;

use crate::background_render::BackgroundRender;
use crate::blur_render::BlurRender;
use crate::camera::Camera;
use crate::console;
use crate::internal_resolution_render::InternalResolutionRender;
use crate::pixels_render::PixelsRender;
use crate::render_types::TextureBufferStack;
use crate::rgb_render::RgbRender;
use crate::simulation_draw::draw;
use crate::simulation_state::{
    InitialParameters, Input, Materials, Resources, SimulationTimers, VideoInputMaterials, VideoInputResources, MOVEMENT_BASE_SPEED, MOVEMENT_SPEED_FACTOR, TURNING_BASE_SPEED,
};
use crate::simulation_update::{change_frontend_input_values, update_simulation};
use crate::wasm_error::WasmResult;
use crate::web_utils::now;

pub fn simulation_tick(input: &mut Input, resources: &mut Resources, materials: &mut Materials) -> WasmResult<bool> {
    pre_process_input(input)?;
    if !update_simulation(resources, input)? {
        console!(log. "User closed the simulation.");
        return Ok(false);
    }
    post_process_input(input);
    if resources.launch_screenshot || resources.screenshot_delay <= 0 {
        draw(materials, resources)?;
    }
    Ok(true)
}

pub fn init_resources(res: &mut Resources, video_input: VideoInputResources) -> WasmResult<()> {
    let now = now()?;
    let initial_position_z = calculate_far_away_position(&video_input);
    let mut camera = Camera::new(MOVEMENT_BASE_SPEED * initial_position_z / MOVEMENT_SPEED_FACTOR, TURNING_BASE_SPEED);
    let mut cur_pixel_width = video_input.pixel_width;
    {
        let res: &Resources = res; // let's avoid using '&mut res' when just reading values
        if res.resetted {
            cur_pixel_width = video_input.pixel_width;
            camera.set_position(glm::vec3(0.0, 0.0, initial_position_z));
        } else {
            let mut camera_position = res.camera.get_position();
            if res.initial_parameters.initial_position_z != camera_position.z {
                camera_position.z = initial_position_z;
            }
            camera.set_position(camera_position);
            if res.crt_filters.cur_pixel_width != res.video.pixel_width {
                cur_pixel_width = res.crt_filters.cur_pixel_width;
            }
        }
    }
    res.resetted = true;
    res.crt_filters.cur_pixel_width = cur_pixel_width;
    res.timers = SimulationTimers {
        frame_count: 0,
        last_time: now,
        last_second: now,
    };
    res.initial_parameters = InitialParameters {
        initial_position_z,
        initial_pixel_width: video_input.pixel_width,
        initial_movement_speed: camera.movement_speed,
    };
    res.crt_filters.internal_resolution.init_viewport_size(video_input.viewport_size);
    res.camera = camera;
    res.video = video_input;
    change_frontend_input_values(res)?;
    Ok(())
}

pub fn load_materials(gl: WebGl2RenderingContext, video: VideoInputMaterials) -> WasmResult<Materials> {
    let pixels_render = PixelsRender::new(&gl, video)?;
    let blur_render = BlurRender::new(&gl)?;
    let internal_resolution_render = InternalResolutionRender::new(&gl)?;
    let rgb_render = RgbRender::new(&gl)?;
    let background_render = BackgroundRender::new(&gl)?;
    let materials = Materials {
        gl,
        main_buffer_stack: TextureBufferStack::new(),
        bg_buffer_stack: TextureBufferStack::new(),
        pixels_render,
        blur_render,
        internal_resolution_render,
        rgb_render,
        background_render,
    };
    Ok(materials)
}

fn calculate_far_away_position(video_input: &VideoInputResources) -> f32 {
    let width = video_input.background_size.width as f32;
    let height = video_input.background_size.height as f32;
    let viewport_width_scaled = (video_input.viewport_size.width as f32 / video_input.pixel_width) as u32;
    let width_ratio = viewport_width_scaled as f32 / width;
    let height_ratio = video_input.viewport_size.height as f32 / height;
    let is_height_bounded = width_ratio > height_ratio;
    let mut bound_ratio = if is_height_bounded { height_ratio } else { width_ratio };
    let mut resolution = if is_height_bounded { video_input.viewport_size.height } else { viewport_width_scaled } as i32;
    while bound_ratio < 1.0 {
        bound_ratio *= 2.0;
        resolution *= 2;
    }
    if !video_input.stretch {
        let mut divisor = bound_ratio as i32;
        while divisor > 1 {
            if resolution % divisor == 0 {
                break;
            }
            divisor -= 1;
        }
        bound_ratio = divisor as f32;
    }
    0.5 + (resolution as f32 / bound_ratio) * if is_height_bounded { 1.2076 } else { 0.68 * video_input.pixel_width }
}

fn pre_process_input(input: &mut Input) -> WasmResult<()> {
    input.now = now()?;
    input.get_mut_fields_booleanbutton().iter_mut().for_each(|button| button.track_input());
    input
        .get_mut_fields_incdec_booleanbutton_()
        .iter_mut()
        .for_each(|incdec| incdec.get_mut_fields_t().iter_mut().for_each(|button| button.track_input()));
    Ok(())
}

fn post_process_input(input: &mut Input) {
    input.mouse_scroll_y = 0.0;
    input.mouse_position_x = 0;
    input.mouse_position_y = 0;
    input.custom_event.kind = String::new();
}