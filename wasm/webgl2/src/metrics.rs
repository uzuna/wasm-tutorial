use std::sync::{atomic::AtomicU32, Arc};

/// WebGLのメトリクスを管理するための構造体です。
///
/// 内部で参照共有をしているためCloneが可能
/// 関連付けられたコンテキストの情報アクセスできます。
#[derive(Default, Clone)]
pub struct Metrics {
    pub shader: Arc<ShaderCount>,
    #[cfg(feature = "vertex")]
    pub vertex: Arc<VertexCount>,
}

impl std::fmt::Display for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Metrics: ")?;
        writeln!(f, "  {}", self.shader)?;
        #[cfg(feature = "vertex")]
        writeln!(f, "  {}", self.vertex)?;
        Ok(())
    }
}

/// シェーダーの数を測定するための構造体です。
#[derive(Default)]
pub struct ShaderCount {
    pub shader_count: AtomicU32,
}

impl ShaderCount {
    pub fn inc_shader(&self, inc: u32) {
        self.shader_count
            .fetch_add(inc, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn sub_shader(&self, sub: u32) {
        self.shader_count
            .fetch_sub(sub, std::sync::atomic::Ordering::Relaxed);
    }
}

impl std::fmt::Display for ShaderCount {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Shaders: {}",
            self.shader_count.load(std::sync::atomic::Ordering::Relaxed)
        )
    }
}

/// 頂点に関する数を測定するための構造体です。
#[derive(Default)]
pub struct VertexCount {
    pub vao_count: AtomicU32,
}

#[cfg(feature = "vertex")]
impl VertexCount {
    pub fn inc_vao(&self, inc: u32) {
        self.vao_count
            .fetch_add(inc, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn sub_vao(&self, sub: u32) {
        self.vao_count
            .fetch_sub(sub, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(feature = "vertex")]
impl std::fmt::Display for VertexCount {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "VertexArrayObjects: {}",
            self.vao_count.load(std::sync::atomic::Ordering::Relaxed)
        )
    }
}
