#![cfg(feature = "context")]
#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use web_sys::WebGlUniformLocation;
use webgl2::{context::Context, error::Result, program::Program};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_pass() -> std::result::Result<(), JsValue> {
    struct Shader {
        program: Program,
        mvp: WebGlUniformLocation,
    }

    impl Shader {
        // versionは開業よりも前になければならない。
        const VERT: &'static str = r#"#version 300 es

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;

uniform mat4 mvp;

out vec4 vertexColor;

void main() {
    vertexColor = color;
    gl_Position = mvp * vec4(position, 1.0);
}
"#;

        const FRAG: &'static str = r#"#version 300 es

precision highp float;

in vec4 vertexColor;
out vec4 fragmentColor;

void main() {
    fragmentColor = vertexColor;
}
"#;

        pub fn new(ctx: &Context) -> Result<Self> {
            let program = ctx.program(Self::VERT, Self::FRAG)?;
            let mvp = program.uniform_location("mvp")?;

            Ok(Self { program, mvp })
        }
    }

    let doc = web_sys::window()
        .ok_or("Failed to get Window")?
        .document()
        .ok_or("Failed to get Document")?;

    let canvas = doc
        .create_element("canvas")
        .expect("Could not create testing node");
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let ctx = webgl2::context::Context::new(canvas, webgl2::context::COLOR_BLACK)?;
    let _s = Shader::new(&ctx);

    Ok(())
}
