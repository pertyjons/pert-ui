//! Renders TextAreas and Labels with literal tabs on the CPU through the
//! public facade and writes PNG captures. CPU evidence only: no native window,
//! GPU or human input is involved.
use pert_ui::cpu::CpuRenderer;
use pert_ui::*;
use std::time::Duration;

fn render(
    element: impl FnOnce(&Runtime) -> Element,
    name: &str,
    out: &std::path::Path,
) -> Result<FrameImage, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(240.0, 90.0, 2.0)?)?;
    let element = element(&ui);
    ui.mount_tree(element)?;
    ui.flush(Duration::ZERO)?;
    let scene = ui.paint()?;
    let image = CpuRenderer::new().render_opaque(&scene)?;
    std::fs::write(out.join(format!("{name}.png")), image.encode_png())?;
    Ok(image)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).ok_or("output directory")?);
    std::fs::create_dir_all(&out)?;
    let area = |text: &'static str| {
        move |ui: &Runtime| Element::new("area", TextArea::new(ui.signal(text.to_owned())))
    };
    let mut images = Vec::new();
    for (name, text) in [
        ("textarea-tab", "A\tB\nName\tValue\n\tindented"),
        ("textarea-space", "A B\nName Value\n indented"),
        ("textarea-private-use", "A\u{E000}B\nName\u{E000}Value\n\u{E000}indented"),
    ] {
        images.push((name, render(area(text), name, &out)?));
    }
    for (name, text) in [
        ("label-body-tab", "Body\ttab\tlabel"),
        ("label-body-private-use", "Body\u{E000}tab\u{E000}label"),
        ("label-body-vertical-tab", "Body\u{0B}tab\u{0B}label"),
    ] {
        images.push((name, render(|_| Element::new("label", Label::new(text)), name, &out)?));
    }
    let same = |a: usize, b: usize| images[a].1.pixels() == images[b].1.pixels();
    println!("textarea-tab == textarea-space (pixel-identical): {}", same(0, 1));
    println!("textarea-tab == textarea-private-use: {}", same(0, 2));
    println!("label-body-tab == label-body-private-use: {}", same(3, 4));
    println!("label-body-private-use == label-body-vertical-tab: {}", same(4, 5));
    for (name, image) in &images {
        println!("{name}.png {}x{}", image.width(), image.height());
    }
    Ok(())
}
