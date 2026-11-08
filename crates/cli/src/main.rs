use anyhow::{anyhow, bail, Result};
use riffle_core::candidate;
use riffle_core::copy;
use riffle_core::decode::{apply_orientation, decode_rgb, decode_upright_near};
use riffle_core::embed;
use riffle_core::eyes;
use riffle_core::faces;
use riffle_core::partial;
use riffle_core::pose;
use riffle_core::reader;
use riffle_core::scan;
use riffle_core::sharpness;
use riffle_core::xmp;
use riffle_core::Flag;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Instant;

mod facechoice;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("info") => info(Path::new(&args[1])),
        Some("focusbox") => focusbox(Path::new(&args[1]), Path::new(&args[2])),
        Some("faces") => faces(Path::new(&args[1]), Path::new(&args[2])),
        Some("bench") => bench(&args[1..]),
        Some("crop") => crop(
            Path::new(&args[1]),
            Path::new(&args[2]),
            args.get(3).map(|s| s.parse()).transpose()?,
        ),
        Some("scan") => scan_dir(Path::new(&args[1]), args.get(2).map(|t| t.parse()).transpose()?),
        Some("candidates") => {
            let mut dirs = &args[1..];
            let threads = match dirs.last().map(|t| t.parse::<usize>()) {
                Some(Ok(n)) => {
                    dirs = &dirs[..dirs.len() - 1];
                    Some(n)
                }
                _ => None,
            };
            if dirs.is_empty() {
                bail!("usage: riffle-cli candidates <dir>... [threads]");
            }
            candidates(&dirs.iter().map(PathBuf::from).collect::<Vec<_>>(), threads)
        }
        Some("detect") => {
            let mut inputs = &args[1..];
            let threads = match inputs.last().map(|t| t.parse::<usize>()) {
                Some(Ok(n)) => {
                    inputs = &inputs[..inputs.len() - 1];
                    Some(n)
                }
                _ => None,
            };
            if inputs.is_empty() {
                bail!("usage: riffle-cli detect <dir|file>... [threads]");
            }
            detect(&inputs.iter().map(PathBuf::from).collect::<Vec<_>>(), threads)
        }
        Some("eyecrops") => {
            let (out, inputs) = match args[1..].split_last() {
                Some((out, inputs)) if !inputs.is_empty() => (out, inputs),
                _ => bail!("usage: riffle-cli eyecrops <dir|file>... <out-dir>"),
            };
            eyecrops(
                &inputs.iter().map(PathBuf::from).collect::<Vec<_>>(),
                Path::new(out),
            )
        }
        Some("eyes") => {
            if args.len() < 2 {
                bail!("usage: riffle-cli eyes <dir|file>...");
            }
            eyes_cmd(&args[1..].iter().map(PathBuf::from).collect::<Vec<_>>())
        }
        Some("features") => {
            let mut dirs = &args[1..];
            let threads = match dirs.last().map(|t| t.parse::<usize>()) {
                Some(Ok(n)) => {
                    dirs = &dirs[..dirs.len() - 1];
                    Some(n)
                }
                _ => None,
            };
            if dirs.is_empty() {
                bail!("usage: riffle-cli features <dir>... [threads]");
            }
            features(&dirs.iter().map(PathBuf::from).collect::<Vec<_>>(), threads)
        }
        Some("check") => {
            let mut dirs = &args[1..];
            let threads = match dirs.last().map(|t| t.parse::<usize>()) {
                Some(Ok(n)) => {
                    dirs = &dirs[..dirs.len() - 1];
                    Some(n)
                }
                _ => None,
            };
            if dirs.is_empty() {
                bail!("usage: riffle-cli check <dir>... [threads]");
            }
            check(&dirs.iter().map(PathBuf::from).collect::<Vec<_>>(), threads)
        }
        Some("facechoice") => facechoice::run(&args[1..]),
        Some("embed") => embed_cmd(&args[1..]),
        Some("previews") => previews(&parse_previews(&args[1..])?),
        Some("copybench") => match &args[1..] {
            [src, dst, rest @ ..] if rest.len() <= 2 => copybench(
                Path::new(src),
                Path::new(dst),
                rest.first().map(|s| s.parse()).transpose()?,
                rest.get(1).map(|s| s.parse()).transpose()?,
            ),
            _ => bail!("usage: {COPYBENCH_USAGE}"),
        },
        Some("embed-compare") => match &args[1..] {
            [a, b] => embed_compare(Path::new(a), Path::new(b)),
            _ => bail!("usage: {EMBED_COMPARE_USAGE}"),
        },
        _ => bail!(
            "usage: riffle-cli <info|focusbox|faces|bench> <file.ARW> [out.png]\n       riffle-cli crop <file.ARW> <out.png> [size]\n       riffle-cli scan <dir> [threads]\n       riffle-cli candidates <dir>... [threads]\n       riffle-cli detect <dir|file>... [threads]\n       riffle-cli eyecrops <dir|file>... <out-dir>\n       riffle-cli eyes <dir|file>...\n       riffle-cli features <dir>... [threads]\n       riffle-cli check <dir>... [threads]\n       {}\n       {EMBED_USAGE}\n       {EMBED_COMPARE_USAGE}\n       {PREVIEWS_USAGE}
       {COPYBENCH_USAGE}",
            facechoice::USAGE
        ),
    }
}

fn info(path: &Path) -> Result<()> {
    let a = reader::read_metadata(path)?;
    println!("orientation: {}", a.orientation);
    println!("preview: {:?}", a.preview);
    println!("full:    {:?}", a.full);
    println!(
        "capture_time: {}",
        a.shot.capture_time.as_deref().unwrap_or("-")
    );
    println!("subsec: {}", a.shot.subsec.as_deref().unwrap_or("-"));
    match a.shot.focus {
        Some(f) => println!("focus: {} {} {} {}", f.sensor_w, f.sensor_h, f.x, f.y),
        None => println!("focus: -"),
    }
    match a.shot.focus_mode {
        Some(m) => println!("focus mode: {m}"),
        None => println!("focus mode: -"),
    }
    Ok(())
}

fn draw_rect(rgb: &mut [u8], w: usize, h: usize, x0: i64, y0: i64, bw: i64, bh: i64) {
    let mut put = |x: i64, y: i64| {
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            let i = (y as usize * w + x as usize) * 3;
            rgb[i] = 255;
            rgb[i + 1] = 0;
            rgb[i + 2] = 0;
        }
    };
    for t in 0..3 {
        for x in x0..x0 + bw {
            put(x, y0 + t);
            put(x, y0 + bh - t);
        }
        for y in y0..y0 + bh {
            put(x0 + t, y);
            put(x0 + bw - t, y);
        }
    }
}

fn focusbox(path: &Path, out: &Path) -> Result<()> {
    let (a, jpeg) = reader::read_preview(path)?;

    let t = Instant::now();
    let (rgb, w, h) = decode_rgb(&jpeg)?;
    println!("preview {w}x{h} decoded in {:?}", t.elapsed());

    let f = a
        .shot
        .focus
        .ok_or_else(|| anyhow!("no FocusLocation in {path:?}"))?;
    let (fw, fh, fx, fy) = (f.sensor_w, f.sensor_h, f.x, f.y);
    let (frame_w, frame_h) = a
        .shot
        .focus_frame
        .map_or((219, 219), |f| (f.width, f.height));
    println!("focus: sensor {fw}x{fh} at ({fx},{fy}) frame {frame_w}x{frame_h}");

    // FocusLocation is in unrotated sensor coordinates, so draw on the unrotated preview first.
    let sx = w as f64 / fw as f64;
    let sy = h as f64 / fh as f64;
    let bw = (frame_w as f64 * sx) as i64;
    let bh = (frame_h as f64 * sy) as i64;
    let mut rgb = rgb;
    draw_rect(
        &mut rgb,
        w,
        h,
        (fx as f64 * sx) as i64 - bw / 2,
        (fy as f64 * sy) as i64 - bh / 2,
        bw,
        bh,
    );

    // Rotate to display orientation, then write it out.
    let (rgb, w, h) = apply_orientation(&rgb, w, h, a.orientation);
    image::save_buffer(out, &rgb, w as u32, h as u32, image::ColorType::Rgb8)?;
    println!("wrote {out:?} ({w}x{h}, orientation {})", a.orientation);
    Ok(())
}

/// Print the focus candidate cue, and draw the searched region (the crop
/// around the AF point, or nothing for the whole image), the AF point, the
/// faces found with their eyes and the region the nearest face was scored
/// over (its sharper mesh eye or the eye window) on the upright preview.
fn faces(path: &Path, out: &Path) -> Result<()> {
    let (a, jpeg) = reader::read_preview(path)?;
    let focus = sharpness::trusted_focus(&a.shot);
    // The first call builds the model; time a second one too.
    let t = Instant::now();
    let d = faces::detect_around(&jpeg, a.orientation, focus)?;
    let first = t.elapsed();
    let t = Instant::now();
    faces::detect_around(&jpeg, a.orientation, focus)?;
    let (w, h) = (d.width, d.height);
    println!(
        "preview {w}x{h}: {} face(s), decode + detection {:?} (first call {first:?})",
        d.faces.len(),
        t.elapsed()
    );
    let cue = candidate::focus_cue(&jpeg, a.orientation, focus)?;
    println!(
        "candidate: {:?}, in focus {}",
        cue.state,
        cue.eye_focus
            .map_or("-".to_string(), |p| format!("{:.0}%", p * 100.0))
    );
    let (mut rgb, _, _) = decode_rgb(&jpeg)?;
    if let Some(f) = cue.face {
        println!(
            "nearest face ({:.0},{:.0}) {:.0}x{:.0}",
            f.x, f.y, f.width, f.height
        );
        let mesh = candidate::meshes_face(&f)
            .then(|| eyes::mesh_of(&rgb, w, h, a.orientation, &f))
            .flatten();
        let m = candidate::eye_focus(&candidate::luma(&rgb, w, h), w, h, &f, mesh.as_ref());
        println!(
            "scored {:?}, lap {:.1}, edge width {}, logit {}",
            m.scored,
            m.lap,
            m.edge_width.map_or("-".to_string(), |e| format!("{e:.2}")),
            m.logit.map_or("-".to_string(), |l| format!("{l:.3}"))
        );
        let r = m.window;
        draw_rect(
            &mut rgb,
            w,
            h,
            r.x as i64,
            r.y as i64,
            r.width as i64 - 1,
            r.height as i64 - 1,
        );
    }
    if let Some((px, py)) = d.point {
        println!("AF point ({px},{py})");
        let r = sharpness::window_at(w, h, px, py, faces::CATCH_CROP);
        draw_rect(
            &mut rgb,
            w,
            h,
            r.x as i64,
            r.y as i64,
            r.width as i64 - 1,
            r.height as i64 - 1,
        );
        draw_rect(&mut rgb, w, h, px as i64 - 10, py as i64 - 10, 20, 20);
    }
    for f in &d.faces {
        println!(
            "face ({:.0},{:.0}) {:.0}x{:.0} score {:.2} eyes ({:.0},{:.0}) ({:.0},{:.0})",
            f.x,
            f.y,
            f.width,
            f.height,
            f.score,
            f.left_eye.0,
            f.left_eye.1,
            f.right_eye.0,
            f.right_eye.1
        );
        draw_rect(
            &mut rgb,
            w,
            h,
            f.x as i64,
            f.y as i64,
            f.width as i64,
            f.height as i64,
        );
        for (x, y) in [f.left_eye, f.right_eye] {
            draw_rect(&mut rgb, w, h, x as i64 - 4, y as i64 - 4, 8, 8);
        }
    }
    let (rgb, w, h) = faces::upright_rgb(&rgb, w, h, a.orientation);
    image::save_buffer(out, &rgb, w as u32, h as u32, image::ColorType::Rgb8)?;
    println!("wrote {out:?} ({w}x{h}, orientation {})", a.orientation);
    Ok(())
}

const CROP_SIZE: usize = 512;

/// The mean, median, p95 and max of a non-empty list of times.
fn summary(mut ms: Vec<f64>) -> (f64, f64, f64, f64) {
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = ms.len();
    (
        ms.iter().sum::<f64>() / n as f64,
        ms[n / 2],
        ms[((n as f64 * 0.95) as usize).min(n - 1)],
        ms[n - 1],
    )
}

fn stats(label: &str, ms: Vec<f64>) {
    let n = ms.len();
    let (mean, median, p95, max) = summary(ms);
    println!(
        "{label:<28} n={n:<4} mean {mean:6.1}ms  median {median:6.1}ms  p95 {p95:6.1}ms  max {max:6.1}ms"
    );
}

fn bench(paths: &[String]) -> Result<()> {
    let mut t_preview = Vec::new();
    let mut t_full = Vec::new();
    let mut t_crop = Vec::new();
    let mut t_faces = Vec::new();
    let mut t_whole = Vec::new();
    let mut t_eyes = Vec::new();

    for p in paths {
        let path = Path::new(p);
        if !scan::is_raw_file(path) {
            bail!("{p}: not a RAW file (bench times .ARW / .CR3 / .DNG / .NEF / .ORF / .RAF only)");
        }
        let a = reader::read_metadata(path)?;

        if a.preview.is_some() {
            let (_, jpeg) = reader::read_preview(path)?;
            let t = Instant::now();
            let (rgb, w, h) = decode_rgb(&jpeg)?;
            t_preview.push(t.elapsed().as_secs_f64() * 1000.0);

            // YuNet is trained on upright faces, so orient before detecting.
            let (rgb, w, h) = apply_orientation(&rgb, w, h, a.orientation);

            if t_faces.is_empty() {
                // Build the model outside the timing.
                faces::detect(&rgb, w, h)?;
                faces::detect_whole(&rgb, w, h)?;
            }
            let t = Instant::now();
            faces::detect(&rgb, w, h)?;
            t_faces.push(t.elapsed().as_secs_f64() * 1000.0);
            let t = Instant::now();
            let found = faces::detect_whole(&rgb, w, h)?;
            t_whole.push(t.elapsed().as_secs_f64() * 1000.0);

            if let Some(face) = found.first() {
                if t_eyes.is_empty() {
                    eyes::ear_of(&rgb, w, h, face)?;
                }
                let t = Instant::now();
                eyes::ear_of(&rgb, w, h, face)?;
                t_eyes.push(t.elapsed().as_secs_f64() * 1000.0);
            }
        }

        if a.full.is_some() {
            let (a, jpeg) = reader::read_full(path)?;
            let t = Instant::now();
            decode_rgb(&jpeg)?;
            t_full.push(t.elapsed().as_secs_f64() * 1000.0);

            // The center fallback the app uses when there is no FocusLocation.
            let t = Instant::now();
            partial::decode_focus_crop(&jpeg, a.shot.focus, CROP_SIZE, CROP_SIZE)?;
            t_crop.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }

    println!();
    if !t_preview.is_empty() {
        stats("1. preview decode", t_preview);
    }
    if !t_full.is_empty() {
        stats("2. JpgFromRaw full decode", t_full);
    }
    if !t_crop.is_empty() {
        stats("3. partial decode 512px", t_crop);
    }
    if !t_faces.is_empty() {
        stats("4. face detection", t_faces);
    }
    if !t_whole.is_empty() {
        stats("5. whole-image detection", t_whole);
    }
    if !t_eyes.is_empty() {
        stats("6. eye state (per face)", t_eyes);
    }
    Ok(())
}

const COPYBENCH_USAGE: &str = "riffle-cli copybench <src-dir> <dst-dir> [chunk-mib] [buffers]";

/// Copy every RAW / JPEG under `src` flat into `dst` through the import's
/// copy pipeline and print the throughput.
fn copybench(
    src: &Path,
    dst: &Path,
    chunk_mib: Option<usize>,
    buffers: Option<usize>,
) -> Result<()> {
    let defaults = copy::Options::default();
    let options = copy::Options {
        chunk: chunk_mib.map_or(defaults.chunk, |m| m << 20),
        buffers: buffers.unwrap_or(defaults.buffers),
    };
    let mut paths = Vec::new();
    let mut failures = Vec::new();
    collect(src, &mut paths, &mut failures);
    for (path, e) in &failures {
        eprintln!("{}: {e}", path.display());
    }
    paths.sort();
    std::fs::create_dir_all(dst)?;
    let jobs: Vec<copy::Job> = paths
        .iter()
        .filter_map(|p| {
            Some(copy::Job {
                src: p.clone(),
                dst: dst.join(p.file_name()?),
            })
        })
        .collect();
    let summary = copy::copy_files_with(&jobs, options, &AtomicBool::new(false), |_| {});
    let mut copied = 0;
    for (job, outcome) in jobs.iter().zip(&summary.results) {
        match outcome {
            copy::Outcome::Copied(_) => copied += 1,
            copy::Outcome::Failed(e) => eprintln!("{}: {e}", job.src.display()),
            copy::Outcome::NotRun => {}
        }
    }
    let secs = summary.elapsed.as_secs_f64();
    println!(
        "chunk {} MiB, {} buffers: {copied} / {} files, {} bytes, {secs:.2} s, {:.1} MB/s",
        options.chunk >> 20,
        options.buffers,
        jobs.len(),
        summary.bytes,
        summary.bytes as f64 / 1e6 / secs.max(1e-9)
    );
    Ok(())
}

/// Extract a whole folder in parallel, with no database: the folder scan
/// throughput benchmark.
fn scan_dir(dir: &Path, threads: Option<usize>) -> Result<()> {
    let threads =
        threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    if threads == 0 {
        bail!("threads must be at least 1");
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| riffle_core::scan::is_raw_file(p))
        .collect();
    paths.sort();
    if paths.is_empty() {
        bail!("no RAW files in {dir:?}");
    }

    let start = Instant::now();
    // Per-file cost is the gap between two completions on the same worker, so
    // each worker's previous completion is kept alongside the totals.
    let last = Mutex::new(vec![start; threads]);
    let acc = Mutex::new((Vec::<f64>::new(), 0usize, 0usize));
    scan::extract_all(
        &paths,
        threads,
        scan::Priority::Normal,
        &scan::ScanFocus::default(),
        |_, r| {
            let now = Instant::now();
            let ms = {
                let mut last = last.lock().unwrap();
                let w = rayon::current_thread_index().unwrap_or(0);
                let ms = now.duration_since(last[w]).as_secs_f64() * 1000.0;
                last[w] = now;
                ms
            };
            let mut acc = acc.lock().unwrap();
            acc.0.push(ms);
            match r {
                Ok(e) => acc.1 += e.thumbnail.len(),
                Err(_) => acc.2 += 1,
            }
        },
        &AtomicBool::new(false),
    )
    .map_err(|e| anyhow!(e))?;
    let total = start.elapsed();

    let (mut ms, bytes, errors) = acc.into_inner().unwrap();
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = paths.len();
    println!(
        "{n} files, {threads} threads, {errors} errors: {:.2}s total, {:.0} files/s",
        total.as_secs_f64(),
        n as f64 / total.as_secs_f64()
    );
    println!(
        "per file on a worker: mean {:.1}ms  p95 {:.1}ms",
        ms.iter().sum::<f64>() / ms.len() as f64,
        ms[((ms.len() as f64 * 0.95) as usize).min(ms.len() - 1)]
    );
    println!(
        "thumbnails: {bytes} bytes total, {} bytes mean",
        bytes / n.max(1)
    );
    Ok(())
}

/// Compute the focus candidate cue of every RAW file in one or more folders
/// in parallel and, over the files with a face whose XMP sidecar holds a
/// pick / reject flag (a pick is in focus), pooled across the folders, print
/// the AUC of the Laplacian alone and of the combined logit, and the precision
/// and coverage of the candidates.
///
/// Each file is one line of whitespace-separated fields: the name, the state,
/// `p` and the probability, `lap` and the scored region's Laplacian variance
/// (the chosen mesh eye's or the eye window's), `edge` and its edge width, the
/// face as `(x,y) WxH` (two fields) or `-` (one), the flag, then the
/// `mesh_columns` (always 25 fields, `-` where a value is missing). A file
/// that failed is `name  error: ...` instead.
fn candidates(dirs: &[PathBuf], threads: Option<usize>) -> Result<()> {
    let threads =
        threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    if threads == 0 {
        bail!("threads must be at least 1");
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let mut found: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| riffle_core::scan::is_raw_file(p))
            .collect();
        if found.is_empty() {
            bail!("no RAW files in {dir:?}");
        }
        found.sort();
        paths.extend(found);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()?;
    // Build the mesh model before the per-file mesh times.
    let probe = faces::Face {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        score: 1.0,
        left_eye: (0.0, 0.0),
        right_eye: (0.0, 0.0),
        nose: (0.0, 0.0),
        mouth_left: (0.0, 0.0),
        mouth_right: (0.0, 0.0),
    };
    eyes::mesh_of(&[0; 3], 1, 1, 1, &probe);

    let start = Instant::now();
    let cues: Vec<_> = pool.install(|| {
        use rayon::prelude::*;
        paths
            .par_iter()
            .map(|p| scan::extract_analysis(p).map(|a| a.cue))
            .collect()
    });
    let total = start.elapsed();
    // The measures behind each face's probability, both mesh eyes' included,
    // for the report only.
    let measured: Vec<Option<Measured>> = pool.install(|| {
        use rayon::prelude::*;
        paths
            .par_iter()
            .zip(&cues)
            .map(|(p, cue)| {
                let face = cue.as_ref().ok()?.face?;
                let (a, jpeg) = reader::read_preview(p).ok()?;
                let (rgb, w, h) = decode_rgb(&jpeg).ok()?;
                let gray = candidate::luma(&rgb, w, h);
                let t = Instant::now();
                let mesh = candidate::meshes_face(&face)
                    .then(|| eyes::mesh_of(&rgb, w, h, a.orientation, &face))
                    .flatten();
                let mesh_ms = t.elapsed().as_secs_f64() * 1000.0;
                Some(Measured {
                    focus: candidate::eye_focus(&gray, w, h, &face, mesh.as_ref()),
                    mesh_ms,
                })
            })
            .collect()
    });

    // `hits` are candidates in focus: the numerator of both rates.
    let (mut labeled, mut cands, mut in_focus, mut hits) = (0, 0, 0, 0);
    let (mut errors, mut faced, mut no_edge) = (0, 0, 0);
    // (in focus, lap, logit) of each labeled faced frame.
    let mut scored: Vec<(bool, f64, Option<f64>)> = Vec::new();
    for ((path, cue), measured) in paths.iter().zip(&cues).zip(&measured) {
        let m = measured.as_ref().map(|m| m.focus);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let flag = std::fs::read(xmp::sidecar_path(path))
            .ok()
            .and_then(|b| xmp::read_flag(&b).ok())
            .filter(|f| *f != Flag::None);
        let cue = match cue {
            Ok(c) => c,
            Err(e) => {
                errors += 1;
                println!("{name}  error: {e}");
                continue;
            }
        };
        let face = cue.face.map_or("-".to_string(), |f| {
            format!("({:.0},{:.0}) {:.0}x{:.0}", f.x, f.y, f.width, f.height)
        });
        let opt =
            |v: Option<f64>, digits: usize| v.map_or("-".to_string(), |v| format!("{v:.digits$}"));
        println!(
            "{name}  {:?}  p {}  lap {}  edge {}  {face}  {}  {}",
            cue.state,
            opt(cue.eye_focus, 3),
            opt(m.map(|m| m.lap), 1),
            opt(m.and_then(|m| m.edge_width), 2),
            flag.map_or("-".to_string(), |f| format!("{f:?}")),
            mesh_columns(cue.face.as_ref(), measured.as_ref())
        );
        if cue.face.is_some() {
            faced += 1;
            no_edge += m.is_some_and(|m| m.edge_width.is_none()) as usize;
        }
        let (Some(flag), Some(m)) = (flag, m) else {
            continue;
        };
        labeled += 1;
        let cand = cue.state == candidate::FocusCandidate::Candidate;
        let pick = flag == Flag::Pick;
        cands += cand as usize;
        in_focus += pick as usize;
        hits += (cand && pick) as usize;
        scored.push((pick, m.lap, m.logit));
    }
    println!(
        "{} files in {} folder(s), {threads} threads, {errors} errors: {:.2}s total",
        paths.len(),
        dirs.len(),
        total.as_secs_f64()
    );
    println!("{faced} with a face, {no_edge} of them with no edge width (probability 0)");
    if labeled > 0 {
        let pct = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
        println!(
            "{labeled} labeled with a face ({in_focus} in focus, {} off): candidates {cands}, in focus {hits} ({:.1}%); in-focus frames {in_focus}, candidates {hits} ({:.1}%)",
            labeled - in_focus,
            pct(hits, cands),
            pct(hits, in_focus)
        );
        let with_edge: Vec<(bool, f64)> = scored
            .iter()
            .filter_map(|&(pick, _, logit)| logit.map(|l| (pick, l)))
            .collect();
        println!(
            "AUC: lap {:.3}, combined {:.3} ({} frames with an edge width)",
            auc(scored.iter().map(|&(pick, lap, _)| (pick, lap))),
            auc(with_edge.iter().copied()),
            with_edge.len()
        );
    }
    Ok(())
}

/// The report-only measures of one faced file of `candidates`.
struct Measured {
    /// The cue's measures, with both mesh eyes' (`None` when the mesh failed).
    focus: candidate::EyeFocus,
    /// The time `eyes::mesh_of` took, in ms.
    mesh_ms: f64,
}

/// The mesh fields of one `candidates` line, 25 whitespace-separated fields:
/// `side` and the face box's long side, `mesh` and the mesh time in ms, `yaw`,
/// `pitch` and `roll` each with its angle in degrees, then per eye (`L`, `R`,
/// the image side) the contour window as `WxH`, its lap and edge width, the
/// iris window as `WxH` and its lap, then `eye`, the scored eye (`L`, `R`, or
/// `-` for the eye window) and why (`sharper`, `only`: the one eye that
/// counted, `window`: the window fallback). `-` for anything missing: no
/// face, no measures, a failed mesh, no pose, a region `None`, no edge width.
fn mesh_columns(face: Option<&faces::Face>, measured: Option<&Measured>) -> String {
    let opt =
        |v: Option<f64>, digits: usize| v.map_or("-".to_string(), |v| format!("{v:.digits$}"));
    let mesh = measured.and_then(|m| m.focus.mesh);
    let pose = mesh.and_then(|m| m.pose);
    let window = |r: Option<candidate::RegionMeasures>| {
        r.map_or("-".to_string(), |r| {
            format!("{}x{}", r.window.width, r.window.height)
        })
    };
    let eye = |e: Option<candidate::EyeRegions>| {
        let (contour, iris) = (e.and_then(|e| e.contour), e.and_then(|e| e.iris));
        format!(
            "{} {} {} {} {}",
            window(contour),
            opt(contour.map(|r| r.lap), 1),
            opt(contour.and_then(|r| r.edge_width), 2),
            window(iris),
            opt(iris.map(|r| r.lap), 1)
        )
    };
    let side = |e: candidate::Eye| match e {
        candidate::Eye::Left => "L",
        candidate::Eye::Right => "R",
    };
    let scored = measured.map_or("- -".to_string(), |m| match m.focus.scored {
        candidate::Scored::Sharper(e) => format!("{} sharper", side(e)),
        candidate::Scored::Only(e) => format!("{} only", side(e)),
        candidate::Scored::Window => "- window".to_string(),
    });
    format!(
        "side {}  mesh {}  yaw {}  pitch {}  roll {}  L {}  R {}  eye {scored}",
        opt(face.map(|f| f.width.max(f.height) as f64), 0),
        opt(measured.map(|m| m.mesh_ms), 1),
        opt(pose.map(|p| p.yaw), 1),
        opt(pose.map(|p| p.pitch), 1),
        opt(pose.map(|p| p.roll), 1),
        eye(mesh.map(|m| m.left)),
        eye(mesh.map(|m| m.right))
    )
}

/// The header of `features`, the column names of `features_line`.
const FEATURES_HEADER: &str = "folder\tfile\tcapture_time\tsubsec\txmp\tdop\tsharpness\tstate\teye_focus\tcue_side\taf\tjudged_side\tear\teyes_open\tyaw\tpitch\troll\teye_offset\tedge_gap\tanalysis_ms\teyes_ms";

/// Dump every feature of every RAW file of the given folders (not recursive,
/// each folder's sorted) on a pool of `threads` threads: `FEATURES_HEADER`,
/// then one tab-separated `features_line` per file, on stdout; the totals go
/// to stderr so stdout stays one table.
///
/// The columns: the folder's name, the file name, the Exif `capture_time`
/// and `subsec`; the XMP and `.dop` flags (`Pick` / `Reject` / `None`, `-`
/// when there is no sidecar, `err` when it is unreadable); from
/// `scan::extract_analysis`, what the scan stores: `sharpness`, the cue
/// `state`, `eye_focus` and the cue face's box side (the longer of width and
/// height) in preview pixels; from the path `eyes_of` takes: `af` / `noaf`
/// (a trusted AF point), the judged face's box side, the EAR of its more
/// closed eye, the eyes-open probability (`1 -` the closed one), `yaw`,
/// `pitch` and `roll` in degrees; from the analysis again, the cue mesh's
/// `eye_offset` and `edge_gap`; then the ms of the analysis and of the eyes
/// path. A missing value is `-`; a failed analysis or eyes path is `err` in
/// its columns.
fn features(dirs: &[PathBuf], threads: Option<usize>) -> Result<()> {
    let threads =
        threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    if threads == 0 {
        bail!("threads must be at least 1");
    }
    let paths = raw_paths(dirs)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()?;
    // Build the models outside the timing.
    let probe = faces::Face {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        score: 1.0,
        left_eye: (0.0, 0.0),
        right_eye: (0.0, 0.0),
        nose: (0.0, 0.0),
        mouth_left: (0.0, 0.0),
        mouth_right: (0.0, 0.0),
    };
    faces::detect(&[0; 3], 1, 1)?;
    faces::detect_whole(&[0; 3], 1, 1)?;
    faces::detect_whole(&[0; 6], 1, 2)?;
    eyes::ear_of(&[0; 3], 1, 1, &probe)?;

    let start = Instant::now();
    let lines: Vec<(String, bool)> = pool.install(|| {
        use rayon::prelude::*;
        paths
            .par_iter()
            .map(|p| {
                let t = Instant::now();
                let analysis = scan::extract_analysis(p);
                let analysis_ms = t.elapsed().as_secs_f64() * 1000.0;
                let t = Instant::now();
                let eyes = eyes_features(p).map_err(|e| format!("{e:#}"));
                let eyes_ms = t.elapsed().as_secs_f64() * 1000.0;
                let failed = analysis.is_err() || eyes.is_err();
                let folder = p
                    .parent()
                    .and_then(Path::file_name)
                    .unwrap_or_default()
                    .to_string_lossy();
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                let line = features_line(
                    &folder,
                    &name,
                    &flag_column(&xmp::sidecar_path(p), xmp::read_flag),
                    &flag_column(
                        &riffle_core::dop::sidecar_path(p),
                        riffle_core::dop::read_flag,
                    ),
                    &analysis,
                    analysis_ms,
                    &eyes,
                    eyes_ms,
                );
                (line, failed)
            })
            .collect()
    });
    let total = start.elapsed();
    println!("{FEATURES_HEADER}");
    let mut errors = 0;
    for (line, failed) in &lines {
        println!("{line}");
        errors += *failed as usize;
    }
    eprintln!(
        "{} files in {} folder(s), {threads} threads, {errors} with an error: {:.2}s total",
        paths.len(),
        dirs.len(),
        total.as_secs_f64()
    );
    Ok(())
}

const EMBED_USAGE: &str = "riffle-cli embed <model.onnx> <siglip2|openclip|tinyclip|mobileclip> <dir|file>... [threads] [--thumb | --near <px>] [--out <out.tsv>] [--previews <dir>]";
const EMBED_COMPARE_USAGE: &str = "riffle-cli embed-compare <a.tsv> <b.tsv>";
/// The neighbors `embed-compare` checks for retrieval agreement.
const EMBED_TOP_K: usize = 5;

/// Embed every RAW file given or in the folders given with the ONNX model at
/// the path given and the candidate spec named (`embed::SPECS`): the full
/// preview, or with `--thumb` the index's thumbnail (`scan::extract`, the
/// 404-px q80 JPEG) decoded again, or with `--near` the preview decoded at
/// the DCT scale nearest that long edge (`decode::decode_upright_near`), with
/// no re-encode. Prints `path\tvec...` per file (to
/// `--out` when given), then the plan build time, the warm ms per image
/// (decode, and embed: preprocessing plus inference), the wall time per image
/// and the peak working set. `--previews` also writes each file's preview
/// JPEG and a `list.tsv` of `path\tjpeg\torientation` for `ref_embed.py`.
fn embed_cmd(args: &[String]) -> Result<()> {
    let mut thumb = false;
    let mut near = None;
    let mut out = None;
    let mut previews = None;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--thumb" => thumb = true,
            "--near" => {
                near = Some(
                    it.next()
                        .ok_or(anyhow!("--near needs a long edge"))?
                        .parse()?,
                )
            }
            "--out" => {
                out = Some(PathBuf::from(
                    it.next().ok_or(anyhow!("--out needs a path"))?,
                ))
            }
            "--previews" => {
                previews = Some(PathBuf::from(
                    it.next().ok_or(anyhow!("--previews needs a folder"))?,
                ))
            }
            _ => rest.push(a.as_str()),
        }
    }
    let threads = match rest.last().map(|t| t.parse::<usize>()) {
        Some(Ok(n)) => {
            rest.pop();
            n
        }
        _ => 1,
    };
    if threads == 0 {
        bail!("threads must be at least 1");
    }
    if thumb && near.is_some() {
        bail!("--thumb and --near are exclusive");
    }
    if previews.is_some() && (thumb || near.is_some()) {
        bail!("--previews writes the full previews; drop --thumb / --near");
    }
    let [model_path, spec_name, inputs @ ..] = rest.as_slice() else {
        bail!("usage: {EMBED_USAGE}");
    };
    if inputs.is_empty() {
        bail!("usage: {EMBED_USAGE}");
    }
    let spec = embed::spec(spec_name).ok_or_else(|| anyhow!("unknown spec {spec_name}"))?;
    let paths = raw_paths(&inputs.iter().map(PathBuf::from).collect::<Vec<_>>())?;
    if let Some(dir) = &previews {
        // Previews are named by file stem in one folder, so a repeat would overwrite.
        let mut stems = std::collections::HashSet::new();
        for p in &paths {
            let stem = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if !stems.insert(stem) {
                bail!(
                    "{}: another input has the same file name; --previews needs unique stems",
                    p.display()
                );
            }
        }
        std::fs::create_dir_all(dir)?;
    }

    let bytes = std::fs::read(model_path)?;
    let t = Instant::now();
    let model = embed::Model::load(&bytes, spec)?;
    let build = t.elapsed();
    drop(bytes);
    let loaded = working_set().map(|w| w.1);
    let load = |p: &Path| -> Result<(Vec<u8>, usize, usize, u16)> {
        if thumb {
            let entry = scan::extract(p).map_err(|e| anyhow!("{e}"))?;
            let (rgb, w, h) = decode_rgb(&entry.thumbnail)?;
            Ok((rgb, w, h, entry.orientation))
        } else {
            let (a, jpeg) = reader::read_preview(p)?;
            if let Some(edge) = near {
                let up = decode_upright_near(&jpeg, a.orientation, edge)?;
                return Ok((up.rgb, up.width, up.height, 1));
            }
            let (rgb, w, h) = decode_rgb(&jpeg)?;
            Ok((rgb, w, h, a.orientation))
        }
    };
    // Warm the plan outside the timing.
    let (rgb, w, h, o) = load(&paths[0])?;
    let t = Instant::now();
    embed::embed_with(&model, &rgb, w, h, o)?;
    let first = t.elapsed();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()?;
    // The load's peak hides the run's, so sample the working set during it.
    let done = AtomicBool::new(false);
    let (results, total, running): (Vec<Result<Embedded>>, _, _) = std::thread::scope(|s| {
        let sampler = s.spawn(|| {
            let mut max = 0;
            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                max = max.max(working_set().map_or(0, |w| w.1));
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            max
        });
        let start = Instant::now();
        let results = pool.install(|| {
            use rayon::prelude::*;
            paths
                .par_iter()
                .map(|p| {
                    let t = Instant::now();
                    let (rgb, w, h, o) = load(p)?;
                    let decode = t.elapsed().as_secs_f64() * 1000.0;
                    let t = Instant::now();
                    let v = embed::embed_with(&model, &rgb, w, h, o)?;
                    Ok((v, decode, t.elapsed().as_secs_f64() * 1000.0, o))
                })
                .collect()
        });
        let total = start.elapsed();
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        (results, total, sampler.join().unwrap_or(0))
    });

    let mut lines = String::new();
    let mut list = String::new();
    let (mut decode, mut embedded, mut failed) = (Vec::new(), Vec::new(), 0);
    for (p, r) in paths.iter().zip(results) {
        match r {
            Ok((v, d, e, o)) => {
                lines.push_str(&embed_line(&p.to_string_lossy(), &v));
                lines.push('\n');
                if let Some(dir) = &previews {
                    // Written here, after the timed run, so the I/O stays out of its numbers.
                    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
                    std::fs::write(dir.join(format!("{stem}.jpg")), reader::read_preview(p)?.1)?;
                    list.push_str(&format!(
                        "{}\t{}\t{o}\n",
                        p.display(),
                        dir.join(format!("{stem}.jpg")).display()
                    ));
                }
                decode.push(d);
                embedded.push(e);
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {e:#}", p.display());
            }
        }
    }
    match &out {
        Some(path) => std::fs::write(path, &lines)?,
        None => print!("{lines}"),
    }
    if let Some(dir) = &previews {
        std::fs::write(dir.join("list.tsv"), list)?;
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let input = match (thumb, near) {
        (true, _) => "thumb".to_string(),
        (_, Some(edge)) => format!("preview near {edge} px"),
        _ => "preview".to_string(),
    };
    let summary = format!(
        "{} {} files ({} input), {threads} threads, {failed} failed\nplan build {:.0} ms, first embed {:.0} ms\nwarm per image: decode {:.1} ms, embed {:.1} ms (mean)\nwall {:.2} s, {:.1} ms/image\nworking set after load {}, during the run up to {:.0} MB, peak {}",
        spec.name,
        paths.len(),
        input,
        build.as_secs_f64() * 1000.0,
        first.as_secs_f64() * 1000.0,
        mean(&decode),
        mean(&embedded),
        total.as_secs_f64(),
        total.as_secs_f64() * 1000.0 / paths.len() as f64,
        loaded.map_or("-".into(), |b| format!("{:.0} MB", b as f64 / 1e6)),
        running as f64 / 1e6,
        working_set().map_or("-".into(), |w| format!("{:.0} MB", w.0 as f64 / 1e6)),
    );
    if out.is_some() {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
    if failed > 0 {
        bail!("{failed} file(s) failed");
    }
    Ok(())
}

const PREVIEWS_USAGE: &str = "riffle-cli previews <out-dir> <dir|file>... --near <px> [threads]";
/// The JPEG quality `previews` re-encodes at.
const PREVIEWS_QUALITY: u8 = 92;

/// `previews`' arguments: the output folder, the inputs, the long edge of
/// the DCT scale and the thread count.
#[derive(Debug, PartialEq)]
struct PreviewsArgs {
    out: PathBuf,
    inputs: Vec<PathBuf>,
    near: usize,
    threads: usize,
}

fn parse_previews(args: &[String]) -> Result<PreviewsArgs> {
    let mut near = None;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--near" => {
                near = Some(
                    it.next()
                        .ok_or(anyhow!("--near needs a long edge"))?
                        .parse::<usize>()?,
                )
            }
            _ => rest.push(a.as_str()),
        }
    }
    let threads = match rest.last().map(|t| t.parse::<usize>()) {
        Some(Ok(n)) if rest.len() > 2 => {
            rest.pop();
            n
        }
        _ => 1,
    };
    if threads == 0 {
        bail!("threads must be at least 1");
    }
    let (Some(near), [out, inputs @ ..]) = (near, rest.as_slice()) else {
        bail!("usage: {PREVIEWS_USAGE}");
    };
    if inputs.is_empty() || near == 0 {
        bail!("usage: {PREVIEWS_USAGE}");
    }
    Ok(PreviewsArgs {
        out: PathBuf::from(out),
        inputs: inputs.iter().map(PathBuf::from).collect(),
        near,
        threads,
    })
}

/// Write every RAW file's preview, decoded at the DCT scale nearest
/// `--near` and turned upright (`decode::decode_upright_near`, the embedding's
/// input), as a JPEG to `<out-dir>/<folder>/<stem>.jpg`, `<folder>` being the
/// RAW file's folder name, and a `list.tsv` of `path\tjpeg\twidth\theight`.
fn previews(a: &PreviewsArgs) -> Result<()> {
    let paths = raw_paths(&a.inputs)?;
    let target = |p: &Path| {
        let folder = p
            .parent()
            .and_then(|d| d.file_name())
            .unwrap_or_default()
            .to_os_string();
        a.out.join(folder).join(format!(
            "{}.jpg",
            p.file_stem().unwrap_or_default().to_string_lossy()
        ))
    };
    let mut seen = std::collections::HashSet::new();
    for p in &paths {
        if !seen.insert(target(p)) {
            bail!(
                "{}: another input has the same folder and file name",
                p.display()
            );
        }
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(a.threads)
        .build()?;
    let start = Instant::now();
    let results: Vec<Result<(PathBuf, usize, usize)>> = pool.install(|| {
        use rayon::prelude::*;
        paths
            .par_iter()
            .map(|p| {
                let (meta, jpeg) = reader::read_preview(p)?;
                let up = decode_upright_near(&jpeg, meta.orientation, a.near)?;
                let mut bytes = Vec::new();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, PREVIEWS_QUALITY)
                    .encode(
                        &up.rgb,
                        up.width as u32,
                        up.height as u32,
                        image::ExtendedColorType::Rgb8,
                    )?;
                let out = target(p);
                std::fs::create_dir_all(out.parent().unwrap_or(&a.out))?;
                std::fs::write(&out, bytes)?;
                Ok((out, up.width, up.height))
            })
            .collect()
    });
    let mut list = String::new();
    let mut failed = 0;
    for (p, r) in paths.iter().zip(results) {
        match r {
            Ok((out, w, h)) => {
                list.push_str(&format!("{}\t{}\t{w}\t{h}\n", p.display(), out.display()))
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {e:#}", p.display());
            }
        }
    }
    std::fs::create_dir_all(&a.out)?;
    std::fs::write(a.out.join("list.tsv"), list)?;
    eprintln!(
        "{} files near {} px, {} threads, {failed} failed: {:.2}s total",
        paths.len(),
        a.near,
        a.threads,
        start.elapsed().as_secs_f64()
    );
    if failed > 0 {
        bail!("{failed} file(s) failed");
    }
    Ok(())
}

/// One file's vector, decode ms, embed ms and the orientation it was fed with.
type Embedded = (Vec<f32>, f64, f64, u16);

/// One `embed` TSV line: the path, then each component.
fn embed_line(path: &str, v: &[f32]) -> String {
    let mut line = path.to_string();
    for x in v {
        line.push_str(&format!("\t{x}"));
    }
    line
}

/// Parse an `embed` TSV (or `ref_embed.py`'s) into `(path, vector)` rows.
fn parse_embeddings(text: &str) -> Result<Vec<(String, Vec<f32>)>> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut cols = l.split('\t');
            let path = cols.next().unwrap_or_default().to_string();
            let v = cols
                .map(|c| c.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow!("{path}: {e}"))?;
            Ok((path, v))
        })
        .collect()
}

fn dot(a: &[f32], b: &[f32]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum()
}

/// The indices of the `k` rows nearest row `i` by dot product, itself excluded.
fn top_k(rows: &[&[f32]], i: usize, k: usize) -> Vec<usize> {
    let mut sims: Vec<(usize, f64)> = (0..rows.len())
        .filter(|&j| j != i)
        .map(|j| (j, dot(rows[i], rows[j])))
        .collect();
    sims.sort_by(|a, b| b.1.total_cmp(&a.1));
    sims.into_iter().take(k).map(|(j, _)| j).collect()
}

/// The per-file cosines of the paths both dumps hold, and the mean share of
/// each file's `k` nearest neighbors within `a` that are also among its `k`
/// nearest within `b`.
fn compare_embeddings(
    a: &[(String, Vec<f32>)],
    b: &[(String, Vec<f32>)],
    k: usize,
) -> Result<(Vec<(String, f64)>, f64)> {
    let bs: std::collections::HashMap<&str, &[f32]> =
        b.iter().map(|(p, v)| (p.as_str(), v.as_slice())).collect();
    let pairs: Vec<(&str, &[f32], &[f32])> = a
        .iter()
        .filter_map(|(p, v)| bs.get(p.as_str()).map(|w| (p.as_str(), v.as_slice(), *w)))
        .collect();
    if let Some((p, v, w)) = pairs.iter().find(|(_, v, w)| v.len() != w.len()) {
        bail!("{p}: vectors differ in length ({} vs {})", v.len(), w.len());
    }
    if let Some((p, v, _)) = pairs.iter().find(|(_, v, _)| v.len() != pairs[0].1.len()) {
        bail!(
            "{p}: row length {} differs from the first row's {}",
            v.len(),
            pairs[0].1.len()
        );
    }
    if pairs.len() <= k {
        bail!("{} common paths; need more than {k}", pairs.len());
    }
    let cosines = pairs
        .iter()
        .map(|(p, v, w)| {
            let cos = dot(v, w) / (dot(v, v) * dot(w, w)).sqrt();
            (p.to_string(), cos)
        })
        .collect();
    let norm = |v: &[f32]| {
        let n = dot(v, v).sqrt() as f32;
        v.iter().map(|x| x / n).collect::<Vec<f32>>()
    };
    let na: Vec<Vec<f32>> = pairs.iter().map(|(_, v, _)| norm(v)).collect();
    let nb: Vec<Vec<f32>> = pairs.iter().map(|(_, _, w)| norm(w)).collect();
    let ra: Vec<&[f32]> = na.iter().map(Vec::as_slice).collect();
    let rb: Vec<&[f32]> = nb.iter().map(Vec::as_slice).collect();
    let agreement = (0..pairs.len())
        .map(|i| {
            let ta = top_k(&ra, i, k);
            let tb = top_k(&rb, i, k);
            ta.iter().filter(|j| tb.contains(j)).count() as f64 / k as f64
        })
        .sum::<f64>()
        / pairs.len() as f64;
    Ok((cosines, agreement))
}

/// Print the per-file cosine between two `embed` dumps of the same files,
/// its min and mean, and the top-`EMBED_TOP_K` retrieval agreement.
fn embed_compare(a: &Path, b: &Path) -> Result<()> {
    let a = parse_embeddings(&std::fs::read_to_string(a)?)?;
    let b = parse_embeddings(&std::fs::read_to_string(b)?)?;
    let (cosines, agreement) = compare_embeddings(&a, &b, EMBED_TOP_K)?;
    for (p, c) in &cosines {
        println!("{p}\t{c:.6}");
    }
    let min = cosines.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
    let mean = cosines.iter().map(|c| c.1).sum::<f64>() / cosines.len() as f64;
    println!(
        "{} common files: cosine min {min:.6} mean {mean:.6}, top-{EMBED_TOP_K} agreement {agreement:.3}",
        cosines.len()
    );
    Ok(())
}

/// The process's peak and current working set in bytes, where the OS
/// reports them.
#[cfg(windows)]
fn working_set() -> Option<(usize, usize)> {
    #[repr(C)]
    struct Counters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, cb: u32) -> i32;
    }
    let cb = std::mem::size_of::<Counters>() as u32;
    let mut c = std::mem::MaybeUninit::<Counters>::zeroed();
    // SAFETY: `c` is a zeroed PROCESS_MEMORY_COUNTERS of size `cb`, and the
    // pseudo handle of the current process needs no closing.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), c.as_mut_ptr(), cb) };
    // SAFETY: zeroed is a valid value of every field, and the call filled it on success.
    (ok != 0).then(|| {
        let c = unsafe { c.assume_init() };
        (c.peak_working_set_size, c.working_set_size)
    })
}

#[cfg(not(windows))]
fn working_set() -> Option<(usize, usize)> {
    None
}

/// The flag column of one sidecar: `-` when it does not exist, `err` when it
/// cannot be read or parsed, else the flag.
fn flag_column(path: &Path, read: fn(&[u8]) -> Result<Flag, String>) -> String {
    match std::fs::read(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "-".into(),
        Err(_) => "err".into(),
        Ok(bytes) => read(&bytes).map_or("err".into(), |f| format!("{f:?}")),
    }
}

/// What the eyes path of one file found, for `features_line`.
struct EyesFeatures {
    capture_time: Option<String>,
    subsec: Option<String>,
    af: bool,
    /// The judged face's box side in preview pixels; `None` without one.
    side: Option<f32>,
    ear: Option<f64>,
    /// The eyes-open probability; `None` when `judge_mesh` gave nothing.
    open: Option<f64>,
    pose: Option<pose::Pose>,
}

/// The eyes judgment of one file, step for step as the app's `read_eyes`:
/// with a trusted AF point one full-size decode for the detection around it
/// and the crop, else `decode_whole` and `detect_whole_upright` then a
/// full-size decode; `judged_face`, then `judge_mesh` on the upright face.
fn eyes_features(path: &Path) -> Result<EyesFeatures> {
    let (a, jpeg) = reader::read_preview(path)?;
    let focus = sharpness::trusted_focus(&a.shot);
    let mut out = EyesFeatures {
        capture_time: a.shot.capture_time.clone(),
        subsec: a.shot.subsec.clone(),
        af: focus.is_some(),
        side: None,
        ear: None,
        open: None,
        pose: None,
    };
    let (found, full) = if focus.is_some() {
        let (rgb, w, h) = decode_rgb(&jpeg)?;
        let found = faces::detect_around_rgb(&rgb, w, h, a.orientation, focus)?;
        (found, Some((rgb, w, h)))
    } else {
        let whole = faces::decode_whole(&jpeg, a.orientation)?;
        (faces::detect_whole_upright(&whole, a.orientation)?, None)
    };
    let Some(&stored) = eyes::judged_face(&found.faces, found.point) else {
        return Ok(out);
    };
    out.side = Some(stored.width.max(stored.height));
    let (rgb, w, h) = match full {
        Some(full) => full,
        None => decode_rgb(&jpeg)?,
    };
    let (upright, uw, uh) = faces::upright_rgb(&rgb, w, h, a.orientation);
    let face = faces::face_to_upright(stored, a.orientation, w, h);
    if let Some(j) = eyes::judge_mesh(&upright, uw, uh, &face) {
        out.ear = eyes::more_closed_ear(&j.points);
        out.open = Some(1.0 - j.eyes.probability);
        out.pose = j.pose;
    }
    Ok(out)
}

/// One tab-separated record of `features` (the columns are on `features`).
#[allow(clippy::too_many_arguments)]
fn features_line(
    folder: &str,
    name: &str,
    xmp_flag: &str,
    dop_flag: &str,
    analysis: &Result<scan::Analysis, String>,
    analysis_ms: f64,
    eyes: &Result<EyesFeatures, String>,
    eyes_ms: f64,
) -> String {
    let opt =
        |v: Option<f64>, digits: usize| v.map_or("-".to_string(), |v| format!("{v:.digits$}"));
    let text = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
    let (analysis, mesh) = match analysis {
        Ok(a) => (
            [
                opt(a.sharpness, 4),
                format!("{:?}", a.cue.state),
                opt(a.cue.eye_focus, 4),
                opt(a.cue.face.map(|f| f.width.max(f.height) as f64), 1),
            ]
            .join("\t"),
            [opt(a.cue.eye_offset, 4), opt(a.cue.edge_gap, 4)].join("\t"),
        ),
        Err(_) => (["err"; 4].join("\t"), ["err"; 2].join("\t")),
    };
    let (time, eyes) = match eyes {
        Ok(e) => (
            [text(&e.capture_time), text(&e.subsec)].join("\t"),
            [
                (if e.af { "af" } else { "noaf" }).to_string(),
                opt(e.side.map(f64::from), 1),
                opt(e.ear, 4),
                opt(e.open, 4),
                opt(e.pose.map(|p| p.yaw), 1),
                opt(e.pose.map(|p| p.pitch), 1),
                opt(e.pose.map(|p| p.roll), 1),
            ]
            .join("\t"),
        ),
        Err(_) => (["err"; 2].join("\t"), ["err"; 7].join("\t")),
    };
    format!("{folder}\t{name}\t{time}\t{xmp_flag}\t{dop_flag}\t{analysis}\t{eyes}\t{mesh}\t{analysis_ms:.1}\t{eyes_ms:.1}")
}

/// One file of `detect`: the path taken, the faces and the times in ms.
struct Detected {
    crop: bool,
    faces: Vec<faces::Face>,
    decode: f64,
    detection: f64,
}

/// Run the second pass's face detection (`detect_around` with
/// `trusted_focus`, so each file takes the crop or whole-image path the scan
/// gives it) on every RAW file given or in the folders given, one line per
/// file, then the totals. One thread unless `threads` says otherwise, so the
/// per-file times are not inflated by contention.
fn detect(inputs: &[PathBuf], threads: Option<usize>) -> Result<()> {
    let threads = threads.unwrap_or(1);
    if threads == 0 {
        bail!("threads must be at least 1");
    }
    let paths = raw_paths(inputs)?;
    // Build the models outside the timing.
    faces::detect(&[0; 3], 1, 1)?;
    faces::detect_whole(&[0; 3], 1, 1)?;
    faces::detect_whole(&[0; 6], 1, 2)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()?;

    let start = Instant::now();
    let results: Vec<Result<Detected>> = pool.install(|| {
        use rayon::prelude::*;
        paths
            .par_iter()
            .map(|p| {
                let (a, jpeg) = reader::read_preview(p)?;
                let focus = sharpness::trusted_focus(&a.shot);
                let (d, decode, detection) = if focus.is_some() {
                    let t = Instant::now();
                    let (rgb, w, h) = decode_rgb(&jpeg)?;
                    let decode = t.elapsed().as_secs_f64() * 1000.0;
                    let t = Instant::now();
                    let d = faces::detect_around_rgb(&rgb, w, h, a.orientation, focus)?;
                    (d, decode, t.elapsed().as_secs_f64() * 1000.0)
                } else {
                    let t = Instant::now();
                    let image = faces::decode_whole(&jpeg, a.orientation)?;
                    let decode = t.elapsed().as_secs_f64() * 1000.0;
                    let t = Instant::now();
                    let d = faces::detect_whole_upright(&image, a.orientation)?;
                    (d, decode, t.elapsed().as_secs_f64() * 1000.0)
                };
                Ok(Detected {
                    crop: focus.is_some(),
                    faces: d.faces,
                    decode,
                    detection,
                })
            })
            .collect()
    });
    let total = start.elapsed();

    let (mut errors, mut faced, mut found) = (0, 0, 0);
    let (mut t_decode, mut t_detect) = (Vec::new(), Vec::new());
    for (path, r) in paths.iter().zip(results) {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match r {
            Ok(Detected {
                crop,
                faces,
                decode,
                detection,
            }) => {
                println!("{}", detect_line(&name, crop, &faces, decode, detection));
                faced += !faces.is_empty() as usize;
                found += faces.len();
                t_decode.push(decode);
                t_detect.push(detection);
            }
            Err(e) => {
                errors += 1;
                println!("{name}  error: {e:#}");
            }
        }
    }
    println!(
        "{} files, {threads} threads, {errors} errors: {:.2}s total",
        paths.len(),
        total.as_secs_f64()
    );
    println!("{faced} with at least one face, {found} faces");
    if !t_detect.is_empty() {
        stats("decode", t_decode);
        stats("detection", t_detect);
    }
    Ok(())
}

/// The RAW files given and those in the folders given, each folder's sorted.
fn raw_paths(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for input in inputs {
        if input.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(input)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| scan::is_raw_file(p))
                .collect();
            if found.is_empty() {
                bail!("no RAW files in {input:?}");
            }
            found.sort();
            paths.extend(found);
        } else if scan::is_raw_file(input) {
            paths.push(input.clone());
        } else {
            bail!("{input:?}: not a folder or a RAW file");
        }
    }
    Ok(paths)
}

/// Run the second pass's face detection (as `detect` does) on every RAW file
/// given or in the folders given and, for every face, write the face crop
/// and the crop of each eye (`eyes::face_square`, `eye_squares`) cut
/// from the full-size upright preview to `out` as PNGs, and one
/// `eyecrops_line` per face to `out/index.txt`.
fn eyecrops(inputs: &[PathBuf], out: &Path) -> Result<()> {
    let paths = raw_paths(inputs)?;
    // The crops are named by file stem, so two inputs sharing one would
    // overwrite each other's PNGs and share an index line.
    let mut stems = std::collections::HashSet::new();
    for p in &paths {
        let stem = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if !stems.insert(stem) {
            bail!("{p:?}: another input has the same file stem; the crops would collide");
        }
    }
    std::fs::create_dir_all(out)?;
    let start = Instant::now();
    let results: Vec<Result<Vec<String>>> = {
        use rayon::prelude::*;
        paths
            .par_iter()
            .map(|p| {
                let (a, jpeg) = reader::read_preview(p)?;
                let focus = sharpness::trusted_focus(&a.shot);
                let d = faces::detect_around(&jpeg, a.orientation, focus)?;
                let nearest = d.point.and_then(|pt| candidate::nearest_face(&d.faces, pt));
                let (rgb, w, h) = decode_rgb(&jpeg)?;
                let (upright, uw, uh) = faces::upright_rgb(&rgb, w, h, a.orientation);
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                let stem = p.file_stem().unwrap_or_default().to_string_lossy();
                let mut lines = Vec::new();
                for (i, f) in d.faces.iter().enumerate() {
                    let face = faces::face_to_upright(*f, a.orientation, w, h);
                    let [left, right] = eye_squares(&face);
                    for (suffix, (center, side)) in [
                        ("face", eyes::face_square(&face)),
                        ("l", left),
                        ("r", right),
                    ] {
                        let (sub, win) = faces::crop_rgb(&upright, uw, uh, center, side);
                        image::save_buffer(
                            out.join(format!("{stem}-{i}-{suffix}.png")),
                            &sub,
                            win.width as u32,
                            win.height as u32,
                            image::ColorType::Rgb8,
                        )?;
                    }
                    lines.push(eyecrops_line(
                        &name,
                        i,
                        focus.is_some(),
                        &face,
                        left.1,
                        nearest == Some(f),
                    ));
                }
                Ok(lines)
            })
            .collect()
    };
    let mut index = String::new();
    let (mut errors, mut found) = (0, 0);
    for (path, r) in paths.iter().zip(results) {
        match r {
            Ok(lines) => {
                found += lines.len();
                for line in lines {
                    index.push_str(&line);
                    index.push('\n');
                }
            }
            Err(e) => {
                errors += 1;
                println!("{}  error: {e:#}", path.display());
            }
        }
    }
    std::fs::write(out.join("index.txt"), index)?;
    println!(
        "{} files, {errors} errors, {found} faces: {:.2}s total",
        paths.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Judge the eyes of the face every RAW file given or in the folders given
/// would be judged on, one thread, one `eyes_line` per file, then the
/// totals: the face nearest the AF point on the scan's detection
/// (`detect_around` with `trusted_focus`), else the largest face at or above
/// `sharpness::FACE_CONFIDENCE`, cut from the full-size upright preview.
/// The EAR and the head pose are printed below `eyes::EYES_MIN_FACE` too,
/// so the labeled set's AUC can be taken over every labeled face.
fn eyes_cmd(inputs: &[PathBuf]) -> Result<()> {
    let paths = raw_paths(inputs)?;
    let probe = faces::Face {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        score: 1.0,
        left_eye: (0.0, 0.0),
        right_eye: (0.0, 0.0),
        nose: (0.0, 0.0),
        mouth_left: (0.0, 0.0),
        mouth_right: (0.0, 0.0),
    };
    // Build the models outside the timing.
    faces::detect(&[0; 3], 1, 1)?;
    faces::detect_whole(&[0; 3], 1, 1)?;
    faces::detect_whole(&[0; 6], 1, 2)?;
    let t = Instant::now();
    eyes::ear_of(&[0; 3], 1, 1, &probe)?;
    println!(
        "first model call (plan build and one run): {:.1}ms",
        t.elapsed().as_secs_f64() * 1000.0
    );
    let start = Instant::now();
    let (mut errors, mut judged) = (0, 0);
    let mut t_model = Vec::new();
    for path in &paths {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let r = judge_file(path);
        match r {
            Ok(Some(Judged {
                crop,
                face,
                ear,
                pose,
                model,
            })) => {
                println!("{}", eyes_line(&name, crop, &face, ear, pose, model));
                judged += 1;
                t_model.push(model);
            }
            Ok(None) => println!("{name}  no face"),
            Err(e) => {
                errors += 1;
                println!("{name}  error: {e:#}");
            }
        }
    }
    println!(
        "{} files, {errors} errors, {judged} faces judged: {:.2}s total",
        paths.len(),
        start.elapsed().as_secs_f64()
    );
    if !t_model.is_empty() {
        stats("eye state model (per face)", t_model);
    }
    Ok(())
}

/// The face `eyes` judged in one file, upright, its EAR, its head pose and
/// the model time.
struct Judged {
    crop: bool,
    face: faces::Face,
    ear: Option<f64>,
    pose: Option<pose::Pose>,
    model: f64,
}

/// `eyes` on one file; `None` when it has no face to judge.
fn judge_file(path: &Path) -> Result<Option<Judged>> {
    let (a, jpeg) = reader::read_preview(path)?;
    let focus = sharpness::trusted_focus(&a.shot);
    let d = faces::detect_around(&jpeg, a.orientation, focus)?;
    let Some(stored) = eyes::judged_face(&d.faces, d.point).copied() else {
        return Ok(None);
    };
    let (rgb, w, h) = decode_rgb(&jpeg)?;
    let (upright, uw, uh) = faces::upright_rgb(&rgb, w, h, a.orientation);
    let face = faces::face_to_upright(stored, a.orientation, w, h);
    let t = Instant::now();
    let points = eyes::landmarks_of(&upright, uw, uh, &face)?;
    let model = t.elapsed().as_secs_f64() * 1000.0;
    Ok(Some(Judged {
        crop: focus.is_some(),
        face,
        ear: points.as_deref().and_then(eyes::more_closed_ear),
        pose: points.and_then(|p| pose::head_pose(&p, uw, uh)),
        model,
    }))
}

/// One file of `eyes`: the path taken, the judged face's box side (the
/// longer of width and height) in preview pixels, the EAR of its more closed
/// eye, the closed probability and the state (`unknown` below
/// `eyes::EYES_MIN_FACE` or without an EAR), the head pose in degrees (`-`
/// without one), and the model time.
fn eyes_line(
    name: &str,
    crop: bool,
    face: &faces::Face,
    ear: Option<f64>,
    pose: Option<pose::Pose>,
    model: f64,
) -> String {
    let side = face.width.max(face.height);
    let judged = ear
        .filter(|_| side >= eyes::EYES_MIN_FACE)
        .map(eyes::Eyes::from_ear);
    format!(
        "{name}  {}  {side:.0}px  ear {}  {}  {}  model {model:.1}ms",
        if crop { "crop" } else { "whole" },
        ear.map_or("-".into(), |e| format!("{e:.4}")),
        match judged {
            Some(e) => format!(
                "{:.2} {}",
                e.probability,
                match e.state {
                    eyes::EyeState::Open => "open",
                    eyes::EyeState::Closed => "closed",
                }
            ),
            None => "- unknown".into(),
        },
        match pose {
            Some(p) => format!("yaw {:.1}  pitch {:.1}  roll {:.1}", p.yaw, p.pitch, p.roll),
            None => "yaw -  pitch -  roll -".into(),
        }
    )
}

/// Side of an eye crop as a fraction of the distance between the two eye
/// points: about 1.8 times the eye's corner-to-corner width, the box Open
/// Model Zoo's eye-state demo cuts.
const EYE_CROP: f32 = 0.8;
/// The smallest eye crop side, in pixels.
const EYE_CROP_MIN: usize = 16;

/// The center and side of the square crop of each eye of an upright `face`
/// (left of the image, then right): centered on the eye point, `EYE_CROP`
/// times the inter-ocular distance, at least `EYE_CROP_MIN`.
fn eye_squares(face: &faces::Face) -> [((usize, usize), usize); 2] {
    let (l, r) = (face.left_eye, face.right_eye);
    let distance = ((r.0 - l.0).powi(2) + (r.1 - l.1).powi(2)).sqrt();
    let side = ((distance * EYE_CROP).round() as usize).max(EYE_CROP_MIN);
    let center = |(x, y): (f32, f32)| (x.round() as usize, y.round() as usize);
    [(center(l), side), (center(r), side)]
}

/// One face of `eyecrops`, on the upright preview: the file, the face
/// number (the `{stem}-{n}-*.png` crops), the path taken, the box side (the
/// longer of width and height) in preview pixels, the score, the eye points,
/// the eye crop side, and whether it is the face nearest the AF point.
fn eyecrops_line(
    name: &str,
    n: usize,
    crop: bool,
    face: &faces::Face,
    eye_side: usize,
    nearest: bool,
) -> String {
    format!(
        "{name}  {n}  {}  {:.0}px  {:.2}  eyes ({:.0},{:.0}) ({:.0},{:.0})  eye crop {eye_side}px{}",
        if crop { "crop" } else { "whole" },
        face.width.max(face.height),
        face.score,
        face.left_eye.0,
        face.left_eye.1,
        face.right_eye.0,
        face.right_eye.1,
        if nearest { "  nearest" } else { "" }
    )
}

/// One file of `detect`: the path taken, the faces (box side, the longer of
/// width and height, in stored preview pixels, and score) and the times.
fn detect_line(
    name: &str,
    crop: bool,
    faces: &[faces::Face],
    decode: f64,
    detection: f64,
) -> String {
    let boxes: Vec<String> = faces
        .iter()
        .map(|f| format!("{:.0}px {:.2}", f.width.max(f.height), f.score))
        .collect();
    format!(
        "{name}  {}  {} face(s){}{}  decode {decode:.1}ms  detection {detection:.1}ms",
        if crop { "crop" } else { "whole" },
        faces.len(),
        if boxes.is_empty() { "" } else { ": " },
        boxes.join(", ")
    )
}

/// The area under the ROC curve of `(in focus, score)` pairs: the share of
/// (in focus, off) pairs where the in-focus frame scores higher, ties 0.5.
fn auc(frames: impl Iterator<Item = (bool, f64)>) -> f64 {
    let (pos, neg): (Vec<_>, Vec<_>) = frames.partition(|(pick, _)| *pick);
    let mut s = 0.0;
    for (_, a) in &pos {
        for (_, b) in &neg {
            s += if a > b {
                1.0
            } else if a == b {
                0.5
            } else {
                0.0
            };
        }
    }
    s / (pos.len() * neg.len()) as f64
}

/// Run every RAW and JPEG file under one or more folders, recursively,
/// through what the app's thumbnail, preview and 1:1 views call, print each
/// failed stage and a per-extension summary, and fail when any file did.
fn check(dirs: &[PathBuf], threads: Option<usize>) -> Result<()> {
    let threads =
        threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    if threads == 0 {
        bail!("threads must be at least 1");
    }
    let mut paths = Vec::new();
    let mut failures = Vec::new();
    let mut folders = 0;
    for dir in dirs {
        if !dir.is_dir() {
            bail!("not a folder: {dir:?}");
        }
        folders += collect(dir, &mut paths, &mut failures);
    }
    paths.sort();
    let unlisted = failures.len();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()?;

    let start = Instant::now();
    let results: Vec<Vec<(&str, String)>> = pool.install(|| {
        use rayon::prelude::*;
        paths.par_iter().map(|p| check_file(p)).collect()
    });
    let total = start.elapsed();

    for (path, errors) in paths.iter().zip(&results) {
        for (stage, e) in errors {
            failures.push((path.clone(), format!("{stage}: {e}")));
        }
    }
    failures.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, e) in &failures {
        println!("{}  {e}", path.display());
    }
    let tallies = tally(
        paths
            .iter()
            .zip(&results)
            .map(|(p, errors)| (p.as_path(), !errors.is_empty())),
    );
    for (ext, (ok, failed)) in &tallies {
        println!("{ext}: {ok} ok, {failed} failed");
    }
    let failed = tallies.values().map(|(_, f)| f).sum::<usize>() + unlisted;
    println!(
        "{} files in {folders} folder(s), {threads} threads, {failed} failed: {:.2}s total",
        paths.len(),
        total.as_secs_f64()
    );
    if failed > 0 {
        bail!("{failed} file(s) failed");
    }
    Ok(())
}

/// Add the RAW and JPEG files under `dir` to `paths`, recursively, and each
/// folder that cannot be listed to `failures`; return the folders listed.
fn collect(dir: &Path, paths: &mut Vec<PathBuf>, failures: &mut Vec<(PathBuf, String)>) -> usize {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            failures.push((dir.to_path_buf(), format!("list: {e}")));
            return 0;
        }
    };
    let mut folders = 1;
    for entry in entries {
        let (path, is_dir) = match entry.and_then(|e| Ok((e.path(), e.file_type()?.is_dir()))) {
            Ok(v) => v,
            Err(e) => {
                failures.push((dir.to_path_buf(), format!("list: {e}")));
                continue;
            }
        };
        // `file_type` does not follow links, so a symlink or junction loop cannot recurse.
        if is_dir {
            folders += collect(&path, paths, failures);
        } else if scan::is_raw_file(&path) || scan::is_jpeg_file(&path) {
            paths.push(path);
        }
    }
    folders
}

/// The first error of each stage the app runs on `path`.
fn check_file(path: &Path) -> Vec<(&'static str, String)> {
    let mut errors = Vec::new();
    if let Err(e) = scan::extract(path) {
        errors.push(("scan", e.message));
    }
    if let Err(e) = reader::read_preview(path).and_then(|(_, jpeg)| decode_rgb(&jpeg)) {
        errors.push(("preview", format!("{e:#}")));
    }
    let full = reader::read_metadata(path).and_then(|a| {
        if a.full.is_none() {
            return Ok(());
        }
        let (a, jpeg) = reader::read_full(path)?;
        partial::decode_focus_crop(&jpeg, a.shot.focus, CROP_SIZE, CROP_SIZE).map(|_| ())
    });
    if let Err(e) = full {
        errors.push(("full", format!("{e:#}")));
    }
    errors
}

/// The (ok, failed) counts of `(path, failed)` pairs per lower-cased extension.
fn tally<'a>(
    files: impl Iterator<Item = (&'a Path, bool)>,
) -> std::collections::BTreeMap<String, (usize, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for (path, failed) in files {
        let ext = path
            .extension()
            .map_or(String::new(), |e| e.to_string_lossy().to_lowercase());
        let c: &mut (usize, usize) = counts.entry(ext).or_default();
        if failed {
            c.1 += 1;
        } else {
            c.0 += 1;
        }
    }
    counts
}

/// Write out the partial decode so it can be checked by eye.
fn crop(path: &Path, out: &Path, size: Option<usize>) -> Result<()> {
    let size = size.unwrap_or(CROP_SIZE);
    let t = Instant::now();
    let (a, jpeg) = reader::read_full(path)?;
    println!(
        "read JpgFromRaw ({} bytes) in {:?}",
        jpeg.len(),
        t.elapsed()
    );

    let t = Instant::now();
    let c = partial::decode_focus_crop(&jpeg, a.shot.focus, size, size)?;
    println!(
        "crop {}x{} at ({},{}) point ({},{}) in {:?}",
        c.crop.width,
        c.crop.height,
        c.crop.x,
        c.crop.y,
        c.point_x,
        c.point_y,
        t.elapsed()
    );

    let rgb: Vec<u8> = c
        .crop
        .pixels
        .chunks_exact(4)
        .flat_map(|p| p[..3].to_vec())
        .collect();
    let (rgb, w, h) = apply_orientation(&rgb, c.crop.width, c.crop.height, a.orientation);
    image::save_buffer(out, &rgb, w as u32, h as u32, image::ColorType::Rgb8)?;
    println!("wrote {out:?} ({w}x{h})");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_takes_the_out_dir_inputs_near_and_threads() {
        let args = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
        assert_eq!(
            parse_previews(&args("out a b --near 640 8")).unwrap(),
            PreviewsArgs {
                out: PathBuf::from("out"),
                inputs: vec![PathBuf::from("a"), PathBuf::from("b")],
                near: 640,
                threads: 8,
            }
        );
        let one = parse_previews(&args("--near 800 out a")).unwrap();
        assert_eq!((one.inputs.len(), one.near, one.threads), (1, 800, 1));
        // With only the out dir and one input, a numeric last argument is the input.
        assert_eq!(
            parse_previews(&args("out 8 --near 640")).unwrap().inputs,
            vec![PathBuf::from("8")]
        );
        for bad in [
            "out a",
            "out --near 640",
            "out a --near",
            "out a --near x",
            "out a b --near 640 0",
            "out a --near 0",
        ] {
            assert!(parse_previews(&args(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn embed_lines_round_trip_and_compare_by_path() {
        let a: Vec<(String, Vec<f32>)> = (0..7)
            .map(|i| {
                let t = i as f32 * 0.3;
                (format!("f{i}"), vec![t.cos(), t.sin()])
            })
            .collect();
        let text: String = a.iter().map(|(p, v)| embed_line(p, v) + "\n").collect();
        assert_eq!(parse_embeddings(&text).unwrap(), a);
        // The same files in another order, one scaled: every cosine is 1 and
        // the neighbors agree.
        let mut b = a.clone();
        b.reverse();
        b[0].1 = b[0].1.iter().map(|x| x * 2.0).collect();
        let (cosines, agreement) = compare_embeddings(&a, &b, 2).unwrap();
        assert_eq!(cosines.len(), 7);
        assert!(cosines.iter().all(|(_, c)| (c - 1.0).abs() < 1e-6));
        assert_eq!(agreement, 1.0);
        // Too few common paths for k.
        assert!(compare_embeddings(&a[..2], &b, 2).is_err());
        // Vectors of different lengths are an error, not a truncated cosine.
        let mut c = a.clone();
        c[3].1.push(0.5);
        assert!(compare_embeddings(&a, &c, 2).is_err());
    }

    #[test]
    fn summary_takes_mean_median_p95_and_max() {
        let ms: Vec<f64> = (1..=20).rev().map(f64::from).collect();
        assert_eq!(summary(ms), (10.5, 11.0, 20.0, 20.0));
        assert_eq!(summary(vec![4.0]), (4.0, 4.0, 4.0, 4.0));
        let ms: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(summary(ms), (50.5, 51.0, 96.0, 100.0));
    }

    #[test]
    fn detect_line_lists_the_path_faces_and_times() {
        let face = |side: f32, score: f32| faces::Face {
            x: 0.0,
            y: 0.0,
            width: side,
            height: side * 1.2,
            score,
            left_eye: (0.0, 0.0),
            right_eye: (0.0, 0.0),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        };
        assert_eq!(
            detect_line("a.DNG", false, &[], 41.26, 15.04),
            "a.DNG  whole  0 face(s)  decode 41.3ms  detection 15.0ms"
        );
        assert_eq!(
            detect_line(
                "b.ARW",
                true,
                &[face(50.0, 0.912), face(40.0, 0.6)],
                30.0,
                9.94
            ),
            "b.ARW  crop  2 face(s): 60px 0.91, 48px 0.60  decode 30.0ms  detection 9.9ms"
        );
    }

    fn upright() -> faces::Face {
        faces::Face {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
            score: 0.9,
            left_eye: (15.0, 25.0),
            right_eye: (35.0, 26.0),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        }
    }

    #[test]
    fn eye_squares_follow_the_inter_ocular_distance() {
        let mut f = upright();
        f.left_eye = (100.0, 50.0);
        f.right_eye = (160.0, 50.0);
        assert_eq!(eye_squares(&f), [((100, 50), 48), ((160, 50), 48)]);
        f.right_eye = (130.0, 90.0);
        assert_eq!(eye_squares(&f)[1], ((130, 90), 40));
        f.right_eye = (110.0, 50.4);
        assert_eq!(
            eye_squares(&f),
            [((100, 50), EYE_CROP_MIN), ((110, 50), EYE_CROP_MIN)]
        );
    }

    #[test]
    fn eyes_line_lists_the_face_its_ear_and_the_judgment() {
        let face = faces::Face {
            width: 80.4,
            height: 96.6,
            ..upright()
        };
        assert_eq!(
            eyes_line("a.ARW", true, &face, Some(0.05), None, 49.04),
            "a.ARW  crop  97px  ear 0.0500  0.93 closed  yaw -  pitch -  roll -  model 49.0ms"
        );
        let pose = pose::Pose {
            yaw: 12.34,
            pitch: -5.06,
            roll: 0.0,
        };
        assert_eq!(
            eyes_line("b.DNG", false, &face, Some(0.3), Some(pose), 50.0),
            "b.DNG  whole  97px  ear 0.3000  0.01 open  yaw 12.3  pitch -5.1  roll 0.0  model 50.0ms"
        );
        let small = faces::Face {
            width: 40.0,
            height: 50.0,
            ..upright()
        };
        assert_eq!(
            eyes_line("c.ARW", true, &small, Some(0.05), Some(pose), 48.0),
            "c.ARW  crop  50px  ear 0.0500  - unknown  yaw 12.3  pitch -5.1  roll 0.0  model 48.0ms"
        );
        assert_eq!(
            eyes_line("d.ARW", true, &face, None, None, 48.0),
            "d.ARW  crop  97px  ear -  - unknown  yaw -  pitch -  roll -  model 48.0ms"
        );
    }

    #[test]
    fn mesh_columns_list_the_pose_and_each_eye() {
        let face = faces::Face {
            width: 80.4,
            height: 96.6,
            ..upright()
        };
        let window = |width, height| sharpness::Window {
            x: 0,
            y: 0,
            width,
            height,
        };
        let region = |w, h, lap, edge_width| candidate::RegionMeasures {
            window: window(w, h),
            lap,
            edge_width,
            edge_width_rel: None,
        };
        let pose = pose::Pose {
            yaw: -32.06,
            pitch: 4.44,
            roll: 0.0,
        };
        let focus = candidate::EyeFocus {
            window: window(30, 14),
            lap: 0.0,
            edge_width: None,
            edge_width_rel: None,
            logit: None,
            probability: 0.0,
            state: candidate::FocusCandidate::NotCandidate,
            scored: candidate::Scored::Sharper(candidate::Eye::Left),
            mesh: Some(candidate::EyeMeasures {
                left: candidate::EyeRegions {
                    contour: Some(region(30, 14, 123.456, Some(2.346))),
                    iris: Some(region(9, 9, 45.0, None)),
                },
                right: candidate::EyeRegions {
                    contour: Some(region(28, 12, 80.0, None)),
                    iris: None,
                },
                ear: None,
                pose: Some(pose),
                eye_offset: None,
                edge_gap: None,
                eye_centers: None,
            }),
        };
        let measured = Measured {
            focus,
            mesh_ms: 35.04,
        };
        assert_eq!(
            mesh_columns(Some(&face), Some(&measured)),
            "side 97  mesh 35.0  yaw -32.1  pitch 4.4  roll 0.0  L 30x14 123.5 2.35 9x9 45.0  R 28x12 80.0 - - -  eye L sharper"
        );
        let only = Measured {
            focus: candidate::EyeFocus {
                scored: candidate::Scored::Only(candidate::Eye::Right),
                ..focus
            },
            ..measured
        };
        assert!(mesh_columns(Some(&face), Some(&only)).ends_with("  eye R only"));
        let failed = Measured {
            focus: candidate::EyeFocus {
                scored: candidate::Scored::Window,
                mesh: None,
                ..focus
            },
            ..measured
        };
        assert_eq!(
            mesh_columns(Some(&face), Some(&failed)),
            "side 97  mesh 35.0  yaw -  pitch -  roll -  L - - - - -  R - - - - -  eye - window"
        );
        assert_eq!(
            mesh_columns(None, None),
            "side -  mesh -  yaw -  pitch -  roll -  L - - - - -  R - - - - -  eye - -"
        );
        assert_eq!(mesh_columns(None, None).split_whitespace().count(), 25);
    }

    #[test]
    fn features_line_lists_every_column_with_dashes_for_missing_values() {
        let face = faces::Face {
            width: 80.4,
            height: 96.6,
            ..upright()
        };
        let analysis = Ok(scan::Analysis {
            cue: candidate::Cue {
                state: candidate::FocusCandidate::Candidate,
                eye_focus: Some(0.912345),
                face: Some(face),
                detection: None,
                eyes_ear: None,
                pose: None,
                eye_offset: Some(0.04567),
                edge_gap: Some(0.25),
                eye_centers: None,
                preview_width: 0,
            },
            sharpness: Some(123.45678),
        });
        let eyes = Ok(EyesFeatures {
            capture_time: Some("2026:09:19 10:11:12".into()),
            subsec: Some("345".into()),
            af: true,
            side: Some(96.6),
            ear: Some(0.21234),
            open: Some(0.98765),
            pose: Some(pose::Pose {
                yaw: -12.34,
                pitch: 4.06,
                roll: 0.0,
            }),
        });
        assert_eq!(
            features_line("2026-09-19", "a.ARW", "Pick", "Pick", &analysis, 40.04, &eyes, 85.06),
            "2026-09-19\ta.ARW\t2026:09:19 10:11:12\t345\tPick\tPick\t123.4568\tCandidate\t0.9123\t96.6\taf\t96.6\t0.2123\t0.9877\t-12.3\t4.1\t0.0\t0.0457\t0.2500\t40.0\t85.1"
        );
        let unknown = Ok(scan::Analysis::default());
        let no_face = Ok(EyesFeatures {
            capture_time: Some("2026:05:22 08:00:00".into()),
            subsec: None,
            af: false,
            side: None,
            ear: None,
            open: None,
            pose: None,
        });
        let line = features_line(
            "2026-05-22",
            "b.DNG",
            "-",
            "None",
            &unknown,
            30.0,
            &no_face,
            50.0,
        );
        assert_eq!(
            line,
            "2026-05-22\tb.DNG\t2026:05:22 08:00:00\t-\t-\tNone\t-\tUnknown\t-\t-\tnoaf\t-\t-\t-\t-\t-\t-\t-\t-\t30.0\t50.0"
        );
        assert_eq!(
            line.split('\t').count(),
            FEATURES_HEADER.split('\t').count()
        );
        let failed = features_line(
            "x",
            "c.ARW",
            "err",
            "-",
            &Err("bad".into()),
            1.0,
            &Err("bad".into()),
            2.0,
        );
        assert_eq!(
            failed,
            "x\tc.ARW\terr\terr\terr\t-\terr\terr\terr\terr\terr\terr\terr\terr\terr\terr\terr\terr\terr\t1.0\t2.0"
        );
    }

    #[test]
    fn eyecrops_line_lists_the_face_and_its_eyes() {
        let face = faces::Face {
            x: 10.0,
            y: 20.0,
            width: 80.4,
            height: 96.6,
            score: 0.876,
            left_eye: (30.2, 50.0),
            right_eye: (70.0, 51.6),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        };
        assert_eq!(
            eyecrops_line("a.ARW", 0, true, &face, 32, true),
            "a.ARW  0  crop  97px  0.88  eyes (30,50) (70,52)  eye crop 32px  nearest"
        );
        assert_eq!(
            eyecrops_line("b.DNG", 2, false, &face, 16, false),
            "b.DNG  2  whole  97px  0.88  eyes (30,50) (70,52)  eye crop 16px"
        );
    }

    #[test]
    fn tally_counts_per_lower_cased_extension() {
        let files = [
            ("a/1.CR3", false),
            ("a/2.cr3", true),
            ("b/3.cr3", false),
            ("b/4.JPG", false),
            ("b/5.arw", true),
        ];
        let counts = tally(files.iter().map(|&(p, f)| (Path::new(p), f)));
        assert_eq!(counts.len(), 3);
        assert_eq!(counts["cr3"], (2, 1));
        assert_eq!(counts["jpg"], (1, 0));
        assert_eq!(counts["arw"], (0, 1));
    }
}
