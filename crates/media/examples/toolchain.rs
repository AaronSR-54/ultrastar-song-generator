use usdx_media::Toolchain;

fn main() {
    let toolchain = Toolchain::detect();
    for tool in [
        toolchain.yt_dlp.as_ref(),
        toolchain.ffmpeg.as_ref(),
        toolchain.ffprobe.as_ref(),
        toolchain.deno.as_ref(),
        toolchain.node.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        println!(
            "{:?}: {} ({})",
            tool.kind,
            tool.version,
            tool.path.display()
        );
    }
    println!("js runtime: {:?}", toolchain.js_runtime_name());

    let issues = toolchain.issues();
    if issues.is_empty() {
        println!("toolchain: ready");
    } else {
        for issue in issues {
            println!("issue: {}: {}", issue.tool, issue.message);
        }
    }
}
