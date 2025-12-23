use image::imageops::FilterType;
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize)]
struct Appearance {
    accent: String,
    bg: String,
    fg: String,
    subfg: String,
    card_bg: String,
    card_border: String,
    shadow: String,
    radius: f32,
    scale: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    wallpaper_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    success: Option<String>,
}

#[derive(Debug, Default)]
struct Options {
    wallpaper: Option<String>,
    wallpaper_file: Option<PathBuf>,
    output: Option<PathBuf>,
    scale: Option<f32>,
    radius: Option<f32>,
    dry_run: bool,
    watch: bool,
    interval_secs: u64,
    overrides: BTreeMap<String, String>,
}

fn main() {
    let options = match parse_args() {
        Ok(options) => options,
        Err(err) => {
            eprintln!("tiss-greetd-appearance: {}", err);
            print_usage();
            std::process::exit(2);
        }
    };

    if options.watch {
        if let Err(err) = run_watch(&options) {
            eprintln!("tiss-greetd-appearance: {}", err);
            std::process::exit(1);
        }
        return;
    }

    if let Err(err) = run_once(&options) {
        eprintln!("tiss-greetd-appearance: {}", err);
        std::process::exit(1);
    }
}

#[derive(Debug)]
struct Palette {
    accent: String,
    bg: String,
    fg: String,
    subfg: String,
    card_bg: String,
    card_border: String,
    shadow: String,
}

fn default_palette() -> Palette {
    Palette {
        accent: "#7cc7ff".to_string(),
        bg: "#0e0f12".to_string(),
        fg: "#f2f4f8".to_string(),
        subfg: "#c2c8d2".to_string(),
        card_bg: "#121620".to_string(),
        card_border: "#232a3a".to_string(),
        shadow: "#12151a".to_string(),
    }
}

fn load_palette(path: &str) -> Option<Palette> {
    let image = image::open(path).ok()?;
    let rgb = image.to_rgb8();
    let resized = image::imageops::resize(&rgb, 64, 64, FilterType::Triangle);

    let mut sum = [0u64; 3];
    let mut count = 0u64;
    let mut best = [0u8; 3];
    let mut best_score = 0.0f32;

    for pixel in resized.pixels() {
        let [r, g, b] = pixel.0;
        sum[0] += r as u64;
        sum[1] += g as u64;
        sum[2] += b as u64;
        count += 1;

        let score = saturation_score(r, g, b);
        if score > best_score {
            best_score = score;
            best = [r, g, b];
        }
    }

    if count == 0 {
        return None;
    }

    let avg = [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    ];

    let bg = mix(avg, [0, 0, 0], 0.85);
    let card_bg = mix(avg, [0, 0, 0], 0.78);
    let card_border = mix(avg, [0, 0, 0], 0.6);
    let shadow = mix(bg, [0, 0, 0], 0.6);
    let fg = choose_fg(bg);
    let subfg = mix(fg, bg, 0.45);

    Some(Palette {
        accent: to_hex(best),
        bg: to_hex(bg),
        fg: to_hex(fg),
        subfg: to_hex(subfg),
        card_bg: to_hex(card_bg),
        card_border: to_hex(card_border),
        shadow: to_hex(shadow),
    })
}

fn saturation_score(r: u8, g: u8, b: u8) -> f32 {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf.max(bf));
    let min = rf.min(gf.min(bf));
    if max == 0.0 {
        return 0.0;
    }
    let sat = (max - min) / max;
    sat * max
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 * (1.0 - t) + b[0] as f32 * t).round() as u8,
        (a[1] as f32 * (1.0 - t) + b[1] as f32 * t).round() as u8,
        (a[2] as f32 * (1.0 - t) + b[2] as f32 * t).round() as u8,
    ]
}

fn luminance(color: [u8; 3]) -> f32 {
    let r = color[0] as f32 / 255.0;
    let g = color[1] as f32 / 255.0;
    let b = color[2] as f32 / 255.0;
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn choose_fg(bg: [u8; 3]) -> [u8; 3] {
    if luminance(bg) < 0.5 {
        [242, 244, 248]
    } else {
        [14, 15, 18]
    }
}

fn to_hex(color: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

fn apply_overrides(palette: &mut Palette, overrides: &BTreeMap<String, String>) {
    if let Some(value) = overrides.get("accent") {
        if let Some(color) = parse_hex_color(value) {
            palette.accent = to_hex(color);
        }
    }
    if let Some(value) = overrides.get("bg") {
        if let Some(color) = parse_hex_color(value) {
            palette.bg = to_hex(color);
        }
    }
    if let Some(value) = overrides.get("fg") {
        if let Some(color) = parse_hex_color(value) {
            palette.fg = to_hex(color);
        }
    }
    if let Some(value) = overrides.get("subfg") {
        if let Some(color) = parse_hex_color(value) {
            palette.subfg = to_hex(color);
        }
    }
    if let Some(value) = overrides.get("card_bg") {
        if let Some(color) = parse_hex_color(value) {
            palette.card_bg = to_hex(color);
        }
    }
    if let Some(value) = overrides.get("card_border") {
        if let Some(color) = parse_hex_color(value) {
            palette.card_border = to_hex(color);
        }
    }
    if let Some(value) = overrides.get("shadow") {
        if let Some(color) = parse_hex_color(value) {
            palette.shadow = to_hex(color);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WallpaperSignature {
    path: Option<String>,
    modified: Option<SystemTime>,
}

fn run_once(options: &Options) -> Result<(), String> {
    let wallpaper = resolve_wallpaper(options);
    let json = build_json(options, wallpaper)?;
    emit_output(options, &json)
}

fn run_watch(options: &Options) -> Result<(), String> {
    let interval = if options.interval_secs == 0 {
        Duration::from_secs(2)
    } else {
        Duration::from_secs(options.interval_secs)
    };
    let mut last_sig: Option<WallpaperSignature> = None;
    loop {
        let wallpaper = resolve_wallpaper(options);
        let sig = signature_for(&wallpaper);
        if last_sig.as_ref() != Some(&sig) {
            let json = build_json(options, wallpaper)?;
            emit_output(options, &json)?;
            last_sig = Some(sig);
        }
        thread::sleep(interval);
    }
}

fn signature_for(wallpaper: &Option<String>) -> WallpaperSignature {
    match wallpaper {
        Some(path) => WallpaperSignature {
            path: Some(path.clone()),
            modified: fs::metadata(path).and_then(|meta| meta.modified()).ok(),
        },
        None => WallpaperSignature {
            path: None,
            modified: None,
        },
    }
}

fn build_json(options: &Options, wallpaper: Option<String>) -> Result<String, String> {
    let mut palette = if let Some(path) = wallpaper.as_ref() {
        match load_palette(path) {
            Some(palette) => palette,
            None => {
                eprintln!(
                    "tiss-greetd-appearance: failed to read wallpaper {}, using defaults",
                    path
                );
                default_palette()
            }
        }
    } else {
        default_palette()
    };

    apply_overrides(&mut palette, &options.overrides);

    let appearance = Appearance {
        accent: palette.accent,
        bg: palette.bg,
        fg: palette.fg,
        subfg: palette.subfg,
        card_bg: palette.card_bg,
        card_border: palette.card_border,
        shadow: palette.shadow,
        radius: options.radius.unwrap_or(18.0),
        scale: options.scale.unwrap_or(1.0),
        wallpaper_path: wallpaper,
        error: options.overrides.get("error").cloned(),
        warning: options.overrides.get("warning").cloned(),
        success: options.overrides.get("success").cloned(),
    };

    serde_json::to_string_pretty(&appearance)
        .map_err(|err| format!("failed to serialize appearance: {}", err))
}

fn emit_output(options: &Options, json: &str) -> Result<(), String> {
    if options.dry_run {
        println!("{}", json);
        return Ok(());
    }

    let output = options.output.clone().unwrap_or_else(default_output_path);
    if let Some(parent) = output.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return Err(format!(
                "failed to create {}: {}",
                parent.display(),
                err
            ));
        }
    }
    fs::write(&output, json)
        .map_err(|err| format!("failed to write {}: {}", output.display(), err))?;
    Ok(())
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "-w" | "--wallpaper" => {
                options.wallpaper = Some(next_arg(&mut args, &arg)?);
            }
            "--wallpaper-file" => {
                options.wallpaper_file = Some(PathBuf::from(next_arg(&mut args, &arg)?));
            }
            "-o" | "--output" => {
                options.output = Some(PathBuf::from(next_arg(&mut args, &arg)?));
            }
            "--scale" => {
                options.scale = Some(parse_float(&next_arg(&mut args, &arg)?)?);
            }
            "--radius" => {
                options.radius = Some(parse_float(&next_arg(&mut args, &arg)?)?);
            }
            "--accent" | "--bg" | "--fg" | "--subfg" | "--card-bg" | "--card-border"
            | "--shadow" | "--error" | "--warning" | "--success" => {
                let key = match arg.as_str() {
                    "--card-bg" => "card_bg",
                    "--card-border" => "card_border",
                    other => other.trim_start_matches("--"),
                };
                options
                    .overrides
                    .insert(key.to_string(), next_arg(&mut args, &arg)?);
            }
            "--print" | "--dry-run" => {
                options.dry_run = true;
            }
            "--watch" => {
                options.watch = true;
            }
            "--interval" => {
                options.interval_secs = parse_int(&next_arg(&mut args, &arg)?)?;
            }
            _ => return Err(format!("unknown option: {}", arg)),
        }
    }
    if options.watch && options.interval_secs == 0 {
        options.interval_secs = 2;
    }
    Ok(options)
}

fn next_arg<I>(args: &mut std::iter::Peekable<I>, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("{} expects a value", flag))
}

fn parse_float(value: &str) -> Result<f32, String> {
    value
        .trim()
        .parse::<f32>()
        .map_err(|_| format!("invalid number: {}", value))
}

fn parse_int(value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid number: {}", value))
}

fn resolve_wallpaper(options: &Options) -> Option<String> {
    if let Some(path) = options.wallpaper.as_ref() {
        return Some(path.clone());
    }
    if let Some(path) = env::var_os("TISS_GREETD_WALLPAPER") {
        let path = path.to_string_lossy().trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    if let Some(file) = options.wallpaper_file.as_ref() {
        if let Ok(content) = fs::read_to_string(file) {
            if let Some(line) = content.lines().next() {
                let line = line.trim();
                if !line.is_empty() {
                    return Some(line.to_string());
                }
            }
        }
    }
    None
}

fn default_output_path() -> PathBuf {
    if let Ok(path) = env::var("XDG_STATE_HOME") {
        if !path.trim().is_empty() {
            return PathBuf::from(path).join("tiss-greetd/appearance.json");
        }
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".local/state/tiss-greetd/appearance.json");
    }
    PathBuf::from("/tmp/tiss-greetd-appearance.json")
}

fn print_usage() {
    println!(
        "Usage: tiss-greetd-appearance [options]\n\
  -w, --wallpaper <path>       Wallpaper image path\n\
      --wallpaper-file <path>  File containing wallpaper path\n\
  -o, --output <path>          Output JSON path\n\
      --scale <float>          UI scale (default: 1.0)\n\
      --radius <float>         Corner radius (default: 18)\n\
      --watch                  Watch for wallpaper changes\n\
      --interval <seconds>     Poll interval when watching (default: 2)\n\
      --accent <hex>           Accent color (#RRGGBB)\n\
      --bg <hex>               Background color\n\
      --fg <hex>               Foreground color\n\
      --subfg <hex>            Secondary text color\n\
      --card-bg <hex>          Card background color\n\
      --card-border <hex>      Card border color\n\
      --shadow <hex>           Shadow color\n\
      --error <hex>            Error color\n\
      --warning <hex>          Warning color\n\
      --success <hex>          Success color\n\
      --print                  Print JSON to stdout\n\
  -h, --help                   Show this help"
    );
}
