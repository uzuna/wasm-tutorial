import init, { start, draw_rs, clear_canvas_rs, set_uniform_color_rs, bind_buffer_rs,create_vbo_rs, get_attr_location_rs,setup_depth_test_rs,create_program_rs, get_context_rs, start_webgl2_texture, create_blendmode_option } from "./pkg/blending.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

const canvas_webgl = document.getElementById("webgl-canvas");
const context = start(canvas_webgl);

const context2 = start_webgl2_texture(document.getElementById("webgl2-canvas"));

const use_rust = false;

const c = document.getElementById("webgl-js");
c.width = 500;
c.height = 300;


// webglコンテキストを取得

if (use_rust) {
    var ctx = get_context_rs(c);
} else {
    var ctx = c.getContext('webgl2')
    ctx.enable(ctx.BLEND);
    ctx.blendFunc(ctx.SRC_ALPHA, ctx.ONE_MINUS_SRC_ALPHA);
}

simple_draw(ctx, use_rust);


const c2 = document.getElementById("webgl2-js");
c2.width = 500;
c2.height = 300;
if (false) {
    var ctx2 = get_context_rs(c2);
} else {
    var ctx2 = c2.getContext('webgl2')
    ctx2.enable(ctx2.BLEND);
    ctx2.blendFunc(ctx2.SRC_ALPHA, ctx2.ONE_MINUS_SRC_ALPHA);
}

texture_draw(ctx2, use_rust);

function simple_draw(ctx, use_rust) {
    if (use_rust) {
        var prg = create_program_rs(ctx);
    }else {
        var v_shader = create_shader(ctx, 'vs_simple');
        var f_shader = create_shader(ctx, 'fs_simple');
        var prg = create_program(ctx, v_shader, f_shader);
    }
    
    if (use_rust) {
        var attr = get_attr_location_rs(ctx, prg, "position");
    }else {
        var attr = get_attr_location(ctx, prg, "position");
    }
    
    
    // 深度テストを有効にする
    if (use_rust) {
        setup_depth_test_rs(ctx);
    } else {
        setup_depth_test(ctx);
    }
    
    // canvasを初期化
    if (use_rust) {
        clear_canvas_rs(ctx);
    } else {
        clear_canvas(ctx);
    }
    
    
    // 頂点の位置
    var position_right = [
        -1.0,  0.5,
        0.5,  0.5,
        -1.0, -1.0,
        0.5, -1.0
    ];
    
    var position_left = [
        -0.5,  1.0,
        1.0,  1.0,
        -0.5, -0.5,
        1.0, -0.5
    ];
    
    draw_rect(ctx, prg, attr, position_right, [1.0, 0.0, 0.0, 0.5]);
    draw_rect(ctx, prg, attr, position_left, [0.0, 1.0, 0.0, 0.5]);   
}

function texture_draw(gl, use_rust) {
    if (use_rust) {
        var prg = create_program_rs(gl);
    }else {
        var v_shader = create_shader(gl, 'vs_coord');
        var f_shader = create_shader(gl, 'fs_coord');
        var prg = create_program(gl, v_shader, f_shader);
    }
    clear_canvas(gl);

    let mat = [
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0
    ];
    set_uniform_window_mat(gl, prg, mat);
    
    // VAO
    var attr_locs = [
        get_attr_location(gl, prg, "position"), 
        get_attr_location(gl, prg, "coord")
    ];
    let attr_strs = [2, 2];
    let vbos = [
        [
            -1.0,  0.5,
            0.5,  0.5,
            -1.0, -1.0,
            0.5, -1.0
        ],
        [
            0.0, 1.0,
            1.0, 1.0,
            0.0, 0.0,
            1.0, 0.0
        ]
    ];
    var vao = create_vao(gl, vbos, attr_locs, attr_strs);

    let texture = color_texture(gl, [255, 0, 0, 128]);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.bindVertexArray(vao);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);

    let texture2 = color_texture(gl, [0, 255, 0, 128]);
    gl.bindTexture(gl.TEXTURE_2D, texture2);
    let vbos2 = [
        [
            -0.5,  1.0,
            1.0,  1.0,
            -0.5, -0.5,
            1.0, -0.5
        ],
        [
            0.0, 1.0,
            1.0, 1.0,
            0.0, 0.0,
            1.0, 0.0
        ]
    ];
    var vao2 = create_vao(gl, vbos2, attr_locs, attr_strs);
    gl.bindVertexArray(vao2);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);

}

// return WebGL Texture
function color_texture(gl, color) {
    var tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(color));
    gl.bindTexture(gl.TEXTURE_2D, null);
    return tex;
}

function draw_rect(ctx, prg, attr, position, color) {

    // ここが非互換原因
    if (use_rust) {
        var vPosition = create_vbo_rs(ctx, position);
    } else {
        var vPosition = create_vbo(ctx, position);
    }

    // VBOをバインドし登録する
    if (use_rust) {
        bind_buffer_rs(ctx, attr, vPosition);
    } else {
        bind_buffer(ctx, attr, vPosition);
    }

    // uniform変数を取得し色を設定
    if (use_rust) {
        set_uniform_color_rs(ctx, prg, color);
    } else {
        set_uniform_color(ctx, prg, color);
    }

    // 描画
    if (use_rust) {
        draw_rs(ctx);
    } else {
        draw(ctx);
    }
}

function get_attr_location(gl, prg, name){
    return gl.getAttribLocation(prg, name);
}

// プログラムオブジェクトを生成しシェーダをリンクする関数
function create_program(gl, vs, fs){
    var program = gl.createProgram();
    
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    
    if(gl.getProgramParameter(program, gl.LINK_STATUS)){
        gl.useProgram(program);
        return program;
    }else{
        alert(gl.getProgramInfoLog(program));
    }
}

function bind_buffer(gl, attr, vbo){
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.enableVertexAttribArray(attr);
    gl.vertexAttribPointer(attr, 2, gl.FLOAT, false, 0, 0);
}

function setup_depth_test(gl){
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.LEQUAL);
}

function set_uniform_window_mat(gl, prg, mat){
    let u = gl.getUniformLocation(prg, 'window_mat');
    gl.uniformMatrix3fv(u, false, new Float32Array(mat));
}


function set_uniform_color(gl, prg, color){
    gl.uniform4fv(gl.getUniformLocation(prg, 'u_color'), color);
}

function clear_canvas(gl){
    gl.clearColor(0.0, 0.0, 0.75, 1.0);
    gl.clearDepth(1.0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
}

function draw(gl){
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
}

// VBOを生成する関数
function create_vbo(gl, data){
    var vbo = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(data), gl.STATIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, null);
    return vbo;
}

// return VAO
function create_vao(gl, vbos, attr_locs, attr_strs){
    var i;
    var vao = gl.createVertexArray();
    gl.bindVertexArray(vao);

    for(i in vbos) {
        var vbo = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vbos[i]), gl.STATIC_DRAW);
        gl.enableVertexAttribArray(attr_locs[i]);
        gl.vertexAttribPointer(attr_locs[i], attr_strs[i], gl.FLOAT, false, 0, 0);
    }
    gl.bindVertexArray(null);
    return vao;
}


// シェーダを生成する関数
function create_shader(gl, id){
    var shader;
    var scriptElement = document.getElementById(id);    
    if(!scriptElement){return;}
    
    switch(scriptElement.type){
        
        case 'x-shader/x-vertex':
            shader = gl.createShader(gl.VERTEX_SHADER);
            break;
            
        case 'x-shader/x-fragment':
            shader = gl.createShader(gl.FRAGMENT_SHADER);
            break;
        default :
            return;
    }
    
    gl.shaderSource(shader, scriptElement.text);
    gl.compileShader(shader);
    
    if(gl.getShaderParameter(shader, gl.COMPILE_STATUS)){        
        return shader;
    }else{
        alert(gl.getShaderInfoLog(shader));
    }
}

const blend_select = document.getElementById("blend-select")
create_blendmode_option(blend_select);

blend_select.addEventListener("change", (e) => {
    console.log("blend select ", e.target.value);
    context.set_blend_mode(e.target.value);
    context2.set_blend_mode(e.target.value);
});
