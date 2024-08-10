import init, { start, draw_rs, clear_canvas_rs, set_uniform_color_rs, bind_buffer_rs,create_vbo_rs, get_attr_location_rs,setup_depth_test_rs,create_program_rs, get_context_rs } from "./pkg/blending.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

const canvas_webgl = document.getElementById("webgl-canvas");
const context = start(canvas_webgl);

const use_rust = false;

const c = document.getElementById("webgl-js");
c.width = 500;
c.height = 300;

// webglコンテキストを取得

if (use_rust) {
    var ctx = get_context_rs(c);
} else {
    var ctx = c.getContext('webgl') || c.getContext('experimental-webgl');
    ctx.enable(ctx.BLEND);
    ctx.blendFunc(ctx.SRC_ALPHA, ctx.ONE_MINUS_SRC_ALPHA);
}

if (use_rust) {
    var prg = create_program_rs(ctx);
}else {
    var v_shader = create_shader(ctx, 'vs');
    var f_shader = create_shader(ctx, 'fs');
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

draw_rect(ctx, position_right, [1.0, 0.0, 0.0, 0.5]);
draw_rect(ctx, position_left, [0.0, 1.0, 0.0, 0.5]);


function draw_rect(ctx, position, color) {

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
