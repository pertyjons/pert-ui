"""Instrument queue writes in an exported benchmark checkout only."""
from pathlib import Path
import sys

renderer = Path(sys.argv[1]) / "crates/pert-ui-wgpu/src"
probe = Path(sys.argv[2])
def insert(path, before, prefix):
    text = path.read_text()
    if text.count(before) != 1:
        raise ValueError(f"unexpected instrumentation site: {before}")
    path.write_text(text.replace(before, prefix + before))

sites = [
    ("lib.rs", "                    self.queue\n                        .write_buffer", "bytes[upload.clone()]", 0),
    ("lib.rs", "            self.queue\n                .write_buffer", "records[upload.clone()]", 1),
    ("strokes.rs", "                queue.write_buffer", "after[upload.clone()]", 2),
    ("textures.rs", "        queue.write_texture", "image.pixels()[..]", 3),
]
if (renderer / "gradients.rs").exists():
    sites.append(("gradients.rs", "            queue.write_buffer(", "bytes[..]", 4))
for file, site, data, kind in sites:
    insert(renderer / file, site, f"crate::probe_upload({kind}, &{data});\n")
with (renderer / "lib.rs").open("a") as out:
    out.write("""
thread_local! {
    static WRITES: std::cell::Cell<[(usize,u64);5]> = const { std::cell::Cell::new([(0,0);5]) };
}
fn probe_upload(kind:usize, data:&[u8]){
    WRITES.update(|mut s|{
        let (len,mut hash)=s[kind];
        for &v in data { hash=(hash ^ u64::from(v)).wrapping_mul(0x100000001b3); }
        s[kind]=(len+data.len(),hash);s
    });
}
/// Returns and resets the writes of the scratch renderer on this thread.
pub fn probe_take_uploads()->[(usize,u64);5] { WRITES.take() }
""")
insert(probe, "        self.present.extend(gpu.present(scene)?);","        pert_ui_wgpu::probe_take_uploads();\n")
insert(probe, "        if let Some(time) = gpu.time(scene)? {",
       '        println!("upload\\t{}\\t{}\\t{:?}", self.name, self.cpu.len(), pert_ui_wgpu::probe_take_uploads());\n')
