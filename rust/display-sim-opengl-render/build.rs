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

use build_tools::{copy_webgl_render_crate_to_file, CopyWebglRenderParams};
fn main() {
    copy_webgl_render_crate_to_file(&CopyWebglRenderParams {
        output_file: "display-sim-webgl-render-copy.rs",
        web_sys_replacement: "crate::opengl_hooks",
        web_error_replacement: "crate::opengl_hooks",
    });
}