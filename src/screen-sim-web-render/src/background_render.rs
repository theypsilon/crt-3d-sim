use crate::web::{WebGl2RenderingContext, WebGlProgram, WebGlVertexArrayObject};

use crate::error::WebResult;
use crate::shaders::{make_quad_vao, make_shader, TEXTURE_VERTEX_SHADER};

pub struct BackgroundRender {
    vao: Option<WebGlVertexArrayObject>,
    shader: WebGlProgram,
    gl: WebGl2RenderingContext,
}

impl BackgroundRender {
    pub fn new(gl: &WebGl2RenderingContext) -> WebResult<BackgroundRender> {
        let shader = make_shader(gl, TEXTURE_VERTEX_SHADER, BACKGROUND_FRAGMENT_SHADER)?;
        let vao = make_quad_vao(gl, &shader)?;
        Ok(BackgroundRender { vao, shader, gl: gl.clone() })
    }

    pub fn render(&self) {
        self.gl.bind_vertex_array(self.vao.as_ref());
        self.gl.use_program(Some(&self.shader));
        self.gl.uniform1i(self.gl.get_uniform_location(&self.shader, "foregroundImage").as_ref(), 0);
        self.gl.uniform1i(self.gl.get_uniform_location(&self.shader, "backgroundImage").as_ref(), 1);
        self.gl
            .draw_elements_with_i32(WebGl2RenderingContext::TRIANGLES, 6, WebGl2RenderingContext::UNSIGNED_INT, 0);
    }
}

pub const BACKGROUND_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;

out vec4 FragColor;
in vec2 TexCoord;

uniform sampler2D foregroundImage;
uniform sampler2D backgroundImage;

void main()
{
    vec4 foregroundColor = texture(foregroundImage, TexCoord);
    float foregroundWeight = (foregroundColor.r + foregroundColor.g + foregroundColor.b + foregroundColor.a) / 4.0;
    vec4 backgroundColor = texture(backgroundImage, TexCoord);
    float backgroundWeight = (backgroundColor.r + backgroundColor.g + backgroundColor.b + backgroundColor.a) / 4.0;
    vec4 result1 = foregroundColor.a * foregroundColor + (1.0 - foregroundColor.a) * backgroundColor;
    float weight1 = (result1.r + result1.g + result1.b + result1.a) / 4.0;
    if (foregroundWeight <= 0.26 && backgroundWeight > foregroundWeight) {
        weight1 = 0.0;
    }
    float factor = weight1 / (weight1 + backgroundWeight * 0.1);
    FragColor = result1 * factor + (1.0 - factor) * backgroundColor;
} 
"#;
