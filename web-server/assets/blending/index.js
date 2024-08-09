import init, { start } from "./pkg/blending.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

const canvas_webgl = document.getElementById("webgl-canvas");
const context = start(canvas_webgl);

const c = document.getElementById("webgl-js");
c.width = 500;
c.height = 300;

// webglコンテキストを取得
var gl = c.getContext('webgl') || c.getContext('experimental-webgl');

// 頂点シェーダとフラグメントシェーダの生成
var v_shader = create_shader('vs');
var f_shader = create_shader('fs');

// プログラムオブジェクトの生成とリンク
var prg = create_program(v_shader, f_shader);

var attr = gl.getAttribLocation(prg, 'position');

// 頂点の位置
var position = [
    -0.5,  1.0,
    1.0,  1.0,
    -0.5, -1.0,
    1.0, -1.0
];

var vPosition = create_vbo(position);

// VBOをバインドし登録する
gl.bindBuffer(gl.ARRAY_BUFFER, vPosition);
gl.enableVertexAttribArray(attr);
gl.vertexAttribPointer(attr, 2, gl.FLOAT, false, 0, 0);

// 深度テストを有効にする
gl.enable(gl.DEPTH_TEST);
gl.depthFunc(gl.LEQUAL);

// canvasを初期化
gl.clearColor(0.0, 0.1, 0.1, 1.0);
gl.clearDepth(1.0);
gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
gl.uniform4fv(gl.getUniformLocation(prg, 'u_color'), [1.0,0.0,0.5,1.0]);

gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);

// プログラムオブジェクトを生成しシェーダをリンクする関数
function create_program(vs, fs){
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

// VBOを生成する関数
function create_vbo(data){
    var vbo = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(data), gl.STATIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, null);
    return vbo;
}

// シェーダを生成する関数
function create_shader(id){
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
