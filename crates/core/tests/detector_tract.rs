//! The ball-game-ranking-features plan's Step 3 shippability check of an
//! object detector: whether its ONNX loads and runs in tract with
//! `default-features = false`, its warm cost on one thread, and its raw
//! outputs on the input tensors the Python reference (`detect_eval.py
//! --tensors`) wrote, so the boxes can be compared with onnxruntime's.
//!
//! Run in release with `--ignored --nocapture` and:
//! - `HANDPIC_DETECTOR_ONNX`: the model file;
//! - `HANDPIC_DETECTOR_INPUT`: the input shape, e.g. `1,3,640,640`;
//! - `HANDPIC_DETECTOR_TENSORS` (optional): a folder of `<name>.f32` input
//!   tensors (little-endian, that shape), each run once, its outputs
//!   written next to it as `<name>.<output label>.f32`.

use std::time::Instant;
use tract_onnx::prelude::*;

const WARM_RUNS: usize = 20;

fn input_shape() -> Vec<usize> {
    std::env::var("HANDPIC_DETECTOR_INPUT")
        .expect("HANDPIC_DETECTOR_INPUT")
        .split(',')
        .map(|d| {
            d.trim()
                .parse()
                .expect("HANDPIC_DETECTOR_INPUT: a comma-separated shape")
        })
        .collect()
}

#[test]
#[ignore]
fn detector_loads_and_runs_in_tract() {
    let path = std::env::var("HANDPIC_DETECTOR_ONNX").expect("HANDPIC_DETECTOR_ONNX");
    let shape = input_shape();
    let t = Instant::now();
    let model = tract_onnx::onnx()
        .with_ignore_value_info(true)
        .with_ignore_output_shapes(true)
        .model_for_path(&path)
        .unwrap()
        .with_input_fact(0, f32::fact(&shape).into())
        .unwrap();
    let labels: Vec<String> = model
        .output_outlets()
        .unwrap()
        .iter()
        .map(|o| model.outlet_label(*o).unwrap_or_default().to_string())
        .collect();
    let plan = model.into_optimized().unwrap().into_runnable().unwrap();
    println!(
        "plan build {:.0} ms, outputs {labels:?}",
        t.elapsed().as_secs_f64() * 1000.0
    );

    let len: usize = shape.iter().product();
    let run = |data: Vec<f32>| {
        let input = tract_ndarray::ArrayD::from_shape_vec(shape.clone(), data).unwrap();
        plan.run(tvec!(Tensor::from(input).into())).unwrap()
    };
    let gray = vec![114.0 / 255.0; len];
    let t = Instant::now();
    run(gray.clone());
    println!("first run {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
    let t = Instant::now();
    for _ in 0..WARM_RUNS {
        run(gray.clone());
    }
    println!(
        "warm {:.1} ms/image on one thread ({WARM_RUNS} runs)",
        t.elapsed().as_secs_f64() * 1000.0 / WARM_RUNS as f64
    );

    let Ok(dir) = std::env::var("HANDPIC_DETECTOR_TENSORS") else {
        return;
    };
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "f32"))
        .filter(|p| {
            p.file_stem()
                .is_some_and(|s| !s.to_string_lossy().contains('.'))
        })
        .collect();
    names.sort();
    for p in &names {
        let bytes = std::fs::read(p).unwrap();
        assert_eq!(bytes.len(), len * 4, "{}: not the input shape", p.display());
        let data = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let outputs = run(data);
        let stem = p.file_stem().unwrap().to_string_lossy();
        for (label, out) in labels.iter().zip(outputs.iter()) {
            let out = out.cast_to::<f32>().unwrap();
            let values = out
                .try_as_plain_ram()
                .unwrap()
                .as_slice::<f32>()
                .unwrap()
                .to_vec();
            let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
            std::fs::write(p.with_file_name(format!("{stem}.{label}.f32")), bytes).unwrap();
        }
    }
    println!("{} tensors run", names.len());
}
