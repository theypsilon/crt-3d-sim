/* Copyright (c) 2019 José manuel Barroso Galindo <theypsilon@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>. */

use crate::web::WebGl2RenderingContext;

use crate::error::WebResult;
use crate::pixels_render::PixelsUniform;
use crate::simulation_render_state::Materials;
use core::simulation_context::SimulationContext;
use core::simulation_core_state::{ColorChannels, Resources, TextureInterpolation};

pub struct SimulationDrawer<'a> {
    ctx: &'a dyn SimulationContext,
    materials: &'a mut Materials,
    res: &'a Resources,
}

impl<'a> SimulationDrawer<'a> {
    pub fn new(ctx: &'a dyn SimulationContext, materials: &'a mut Materials, res: &'a Resources) -> Self {
        materials.gl.enable(WebGl2RenderingContext::DEPTH_TEST);
        SimulationDrawer { ctx, materials, res }
    }

    pub fn draw(&mut self) -> WebResult<()> {
        let gl = &self.materials.gl;

        // @TODO: Delete this, only activated for testing in slow computer
        //return Ok(());

        if self.res.video.needs_buffer_data_load {
            self.materials.pixels_render.load_image(&self.res.video);
        }

        self.materials.main_buffer_stack.set_depthbuffer(self.res.output.pixel_have_depth);

        self.materials
            .main_buffer_stack
            .set_resolution(self.res.filters.internal_resolution.width(), self.res.filters.internal_resolution.height());

        self.materials
            .main_buffer_stack
            .set_interpolation(match self.res.filters.texture_interpolation {
                TextureInterpolation::Linear => WebGl2RenderingContext::LINEAR,
                TextureInterpolation::Nearest => WebGl2RenderingContext::NEAREST,
            });

        self.materials.main_buffer_stack.push()?;
        self.materials.main_buffer_stack.push()?;
        self.materials.main_buffer_stack.bind_current()?;

        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

        let view = self.res.camera.get_view();
        let position = self.res.camera.get_position();
        let projection = self
            .res
            .camera
            .get_projection(self.res.video.viewport_size.width as f32, self.res.video.viewport_size.height as f32);

        for hl_idx in 0..self.res.filters.horizontal_lpp {
            for vl_idx in 0..self.res.filters.vertical_lpp {
                for color_idx in 0..self.res.output.color_splits {
                    if let ColorChannels::Overlapping = self.res.filters.color_channels {
                        self.materials.main_buffer_stack.push()?;
                        self.materials.main_buffer_stack.bind_current()?;
                        if vl_idx == 0 && hl_idx == 0 {
                            gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
                        }
                    }
                    self.materials.pixels_render.render(PixelsUniform {
                        shadow_kind: self.res.filters.pixel_shadow_shape_kind.value,
                        geometry_kind: self.res.filters.pixels_geometry_kind,
                        view: view.as_slice(),
                        projection: projection.as_slice(),
                        ambient_strength: self.res.output.ambient_strength,
                        contrast_factor: self.res.filters.extra_contrast,
                        light_color: &self.res.output.light_color[color_idx],
                        extra_light: &self.res.output.extra_light,
                        light_pos: position.as_slice(),
                        screen_curvature: self.res.output.screen_curvature_factor,
                        pixel_spread: &self.res.output.pixel_spread,
                        pixel_scale: &self
                            .res
                            .output
                            .pixel_scale_foreground
                            .get(vl_idx * self.res.filters.horizontal_lpp + hl_idx)
                            .expect("Bad pixel_scale_foreground")[color_idx],
                        pixel_pulse: self.res.output.pixels_pulse,
                        pixel_offset: &self
                            .res
                            .output
                            .pixel_offset_foreground
                            .get(vl_idx * self.res.filters.horizontal_lpp + hl_idx)
                            .expect("Bad pixel_offset_foreground")[color_idx],
                        height_modifier_factor: self.res.output.height_modifier_factor,
                    });
                }
                if let ColorChannels::Overlapping = self.res.filters.color_channels {
                    self.materials.main_buffer_stack.pop()?;
                    self.materials.main_buffer_stack.pop()?;
                    self.materials.main_buffer_stack.pop()?;
                }
            }
        }

        if let ColorChannels::Overlapping = self.res.filters.color_channels {
            self.materials.main_buffer_stack.bind_current()?;
            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);
            gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.materials.main_buffer_stack.get_nth(1)?.texture());
            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 1);
            gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.materials.main_buffer_stack.get_nth(2)?.texture());
            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 2);
            gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.materials.main_buffer_stack.get_nth(3)?.texture());

            self.materials.rgb_render.render();

            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);
        }

        self.materials.main_buffer_stack.push()?;
        self.materials.main_buffer_stack.bind_current()?;
        gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

        if self.res.output.showing_background {
            self.materials.bg_buffer_stack.set_resolution(1920 / 2, 1080 / 2);
            self.materials.bg_buffer_stack.set_depthbuffer(false);
            self.materials.bg_buffer_stack.set_interpolation(WebGl2RenderingContext::LINEAR);
            self.materials.bg_buffer_stack.push()?;
            self.materials.bg_buffer_stack.bind_current()?;
            gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
            for hl_idx in 0..self.res.filters.horizontal_lpp {
                for vl_idx in 0..self.res.filters.vertical_lpp {
                    self.materials.pixels_render.render(PixelsUniform {
                        shadow_kind: 0,
                        geometry_kind: self.res.filters.pixels_geometry_kind,
                        view: view.as_slice(),
                        projection: projection.as_slice(),
                        ambient_strength: self.res.output.ambient_strength,
                        contrast_factor: self.res.filters.extra_contrast,
                        light_color: &self.res.output.light_color_background,
                        extra_light: &[0.0, 0.0, 0.0],
                        light_pos: position.as_slice(),
                        pixel_spread: &self.res.output.pixel_spread,
                        pixel_scale: &self.res.output.pixel_scale_background[vl_idx * self.res.filters.horizontal_lpp + hl_idx],
                        screen_curvature: self.res.output.screen_curvature_factor,
                        pixel_pulse: self.res.output.pixels_pulse,
                        pixel_offset: &self.res.output.pixel_offset_background[vl_idx * self.res.filters.horizontal_lpp + hl_idx],
                        height_modifier_factor: 0.0,
                    });
                }
            }
            let source = self.materials.bg_buffer_stack.get_current()?.clone();
            let target = self.materials.main_buffer_stack.get_current()?;
            self.materials.blur_render.render(&mut self.materials.bg_buffer_stack, &source, &target, 6)?;
            self.materials.bg_buffer_stack.pop()?;
        }
        self.materials.main_buffer_stack.pop()?;
        self.materials.main_buffer_stack.pop()?;
        self.materials.main_buffer_stack.bind_current()?;
        gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

        gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.materials.main_buffer_stack.get_nth(1)?.texture());
        gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 1);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.materials.main_buffer_stack.get_nth(2)?.texture());
        self.materials.background_render.render();
        gl.active_texture(WebGl2RenderingContext::TEXTURE0 + 0);

        if self.res.filters.blur_passes > 0 {
            let target = self.materials.main_buffer_stack.get_current()?.clone();
            self.materials
                .blur_render
                .render(&mut self.materials.main_buffer_stack, &target, &target, self.res.filters.blur_passes)?;
        }

        self.materials.screenshot_pixels = None;
        if self.res.screenshot_trigger.is_triggered {
            let width = self.res.filters.internal_resolution.width();
            let height = self.res.filters.internal_resolution.height();
            let mut pixels: Box<[u8]> = vec![0; (width * height * 4) as usize].into_boxed_slice();
            gl.read_pixels_with_opt_u8_array(
                0,
                0,
                width,
                height,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                Some(&mut *pixels),
            )?;
            self.materials.screenshot_pixels = Some(pixels);
            self.ctx.dispatcher().dispatch_screenshot(
                self.materials.screenshot_pixels.as_ref().expect("Screenshot bug"),
                self.res.filters.internal_resolution.multiplier,
            );
        }

        self.materials.main_buffer_stack.pop()?;
        self.materials.main_buffer_stack.assert_no_stack()?;

        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
        gl.viewport(0, 0, self.res.video.viewport_size.width as i32, self.res.video.viewport_size.height as i32);

        self.materials
            .internal_resolution_render
            .render(self.materials.main_buffer_stack.get_nth(1)?.texture());

        check_error(&gl, line!())?;

        Ok(())
    }
}

fn check_error(gl: &WebGl2RenderingContext, line: u32) -> WebResult<()> {
    let error = gl.get_error();
    if error != WebGl2RenderingContext::NO_ERROR {
        return Err(format!("{} on line: {}", error, line).into());
    }
    Ok(())
}
