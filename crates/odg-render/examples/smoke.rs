//! cargo run -p odg-render --example smoke -- "flowchart LR\n A --> B"
use odg_core::{analyze, AnalyzeOptions, MermaidEngine};

fn main() {
    let src = std::env::args().nth(1).unwrap_or_else(|| "flowchart LR\n  A[Start] --> B{Ok?}\n  C[Orphan]".into()).replace("\\n", "\n");
    let started = std::time::Instant::now();
    let out = odg_render::run(move |r| {
        let a = analyze(&MermaidEngine, &src, &r, &AnalyzeOptions::default());
        if let Some(png) = &a.annotated_png {
            std::fs::write(std::env::temp_dir().join("odg-smoke-annotated.png"), png).unwrap();
        }
        if let Some(png) = &a.png {
            std::fs::write(std::env::temp_dir().join("odg-smoke.png"), png).unwrap();
        }
        a.report.to_markdown()
    });
    println!("{}\n({} ms)", out.unwrap(), started.elapsed().as_millis());
}
