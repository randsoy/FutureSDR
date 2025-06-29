use anyhow::Result;
use futuresdr::blocks::Copy;
use futuresdr::blocks::VectorSink;
use futuresdr::blocks::VectorSinkBuilder;
use futuresdr::blocks::VectorSource;
use futuresdr::blocks::VulkanBuilder;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use futuresdr::runtime::buffer::vulkan::Broker;
use futuresdr::runtime::buffer::vulkan::D2H;
use futuresdr::runtime::buffer::vulkan::H2D;
use futuresdr::tracing::info;
use std::iter::repeat_with;
use std::sync::Arc;

pub fn run_fg() -> Result<()> {
    let mut fg = Flowgraph::new();

    let n_items = 1_000;
    let orig: Vec<f32> = repeat_with(rand::random::<f32>).take(n_items).collect();
    let broker = Arc::new(Broker::new());

    let src = VectorSource::<f32>::new(orig.clone());
    let copy = Copy::<f32>::new();
    let vulkan = VulkanBuilder::new(broker).build();
    let snk = VectorSinkBuilder::<f32>::new().build();

    let src = fg.add_block(src)?;
    let copy = fg.add_block(copy)?;
    let vulkan = fg.add_block(vulkan)?;
    let snk = fg.add_block(snk)?;

    fg.connect_stream(src, "out", copy, "in")?;
    fg.connect_stream_with_type(copy, "out", vulkan, "in", H2D::new())?;
    fg.connect_stream_with_type(vulkan, "out", snk, "in", D2H::new())?;

    fg = Runtime::new().run(fg)?;

    let snk = fg.kernel::<VectorSink<f32>>(snk).unwrap();
    let v = snk.items();

    assert_eq!(v.len(), n_items);
    for i in 0..v.len() {
        assert!((orig[i] * 12.0 - v[i]).abs() < f32::EPSILON);
    }
    info!("data matches");
    info!("first items {:?}", &v[0..10]);

    Ok(())
}

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use jni::JNIEnv;
    use jni::objects::JClass;
    use jni::objects::JString;

    #[allow(non_snake_case)]
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_net_bastibl_futuresdr_MainActivity_runFg(
        mut env: JNIEnv,
        _class: JClass,
        tmp_dir: JString,
    ) {
        let dir: String = env
            .get_string(&tmp_dir)
            .expect("Couldn't get java string!")
            .into();
        unsafe {
            std::env::set_var("FUTURESDR_tmp_dir", dir);
        }
        run_fg().unwrap();
    }
}
