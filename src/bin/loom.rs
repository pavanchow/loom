//! Loom command line tool.
//!
//! Usage:
//!   loom [WIDTH HEIGHT]   Lay out the sample UI at the given size and print the
//!                         computed rectangle tree. Defaults to 800 x 600.
//!   loom demo            Lay out the sample UI and also print the recorded draw
//!                         calls and a hit test probe.

use loom::prelude::*;
use loom::{format_tree, sample_ui};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("demo") {
        run_demo();
        return;
    }

    let (w, h) = match args.as_slice() {
        [] => (800.0, 600.0),
        [w, h] => (parse(w, 800.0), parse(h, 600.0)),
        _ => {
            eprintln!("usage: loom [WIDTH HEIGHT] | loom demo");
            std::process::exit(2);
        }
    };

    let mut root = sample_ui();
    assign_ids(&mut root);
    compute_layout(&mut root, Size::new(w, h));

    println!("Loom layout at {w} x {h}");
    print!("{}", format_tree(&root));
}

fn run_demo() {
    let w = 800.0;
    let h = 600.0;
    let mut root = sample_ui();
    assign_ids(&mut root);
    compute_layout(&mut root, Size::new(w, h));

    println!("== Computed rectangle tree ==");
    print!("{}", format_tree(&root));

    println!("\n== Recorded draw calls ==");
    let mut rec = RecordingRenderer::new();
    render(&root, &mut rec);
    for call in &rec.calls {
        match call {
            DrawCall::Rect { id, kind, rect } => println!(
                "draw_rect  #{id} {kind} ({:.1}, {:.1}, {:.1}, {:.1})",
                rect.x, rect.y, rect.w, rect.h
            ),
            DrawCall::Text { id, rect, text } => println!(
                "draw_text  #{id} {text:?} at ({:.1}, {:.1})",
                rect.x, rect.y
            ),
        }
    }

    println!("\n== Hit test ==");
    let (px, py) = (40.0, 30.0);
    match hit_test(&root, px, py) {
        Some(id) => println!("point ({px}, {py}) hits node #{id}"),
        None => println!("point ({px}, {py}) hits nothing"),
    }
    println!("hit path: {:?}", hit_path(&root, px, py));
}

fn parse(s: &str, fallback: f64) -> f64 {
    s.parse::<f64>().unwrap_or(fallback)
}
