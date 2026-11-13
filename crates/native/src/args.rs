use std::{ffi::OsString, path::PathBuf};

pub const USAGE: &str = "usage: riffle-native [<folder>] [--auto N] [--size WxH] [--max N] [--index <path>] [--settings <path>] [--import-settings <path>] [--data-dir <path>] [--log <path>]";

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub folder: Option<PathBuf>,
    pub auto: Option<usize>,
    pub size: Option<(u32, u32)>,
    pub max: Option<usize>,
    pub index: Option<PathBuf>,
    pub settings: Option<PathBuf>,
    pub import_settings: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub log: Option<PathBuf>,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| -> Result<String, String> {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            value
                .into_string()
                .map_err(|_| format!("{name}: the value is not valid Unicode"))
        };
        match arg.to_str() {
            Some("--auto") => parsed.auto = Some(number("--auto", &value("--auto")?)?),
            Some("--max") => parsed.max = Some(number("--max", &value("--max")?)?),
            Some("--size") => parsed.size = Some(size(&value("--size")?)?),
            Some("--index") => parsed.index = Some(PathBuf::from(value("--index")?)),
            Some("--settings") => parsed.settings = Some(PathBuf::from(value("--settings")?)),
            Some("--import-settings") => {
                parsed.import_settings = Some(PathBuf::from(value("--import-settings")?))
            }
            Some("--data-dir") => parsed.data_dir = Some(PathBuf::from(value("--data-dir")?)),
            Some("--log") => parsed.log = Some(PathBuf::from(value("--log")?)),
            Some(flag) if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            _ if parsed.folder.is_none() => parsed.folder = Some(PathBuf::from(arg)),
            _ => return Err(format!("unexpected argument {}", arg.to_string_lossy())),
        }
    }
    Ok(parsed)
}

fn number<T: std::str::FromStr>(name: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{name}: not a number: {value}"))
}

fn size(value: &str) -> Result<(u32, u32), String> {
    let (w, h) = value
        .split_once('x')
        .ok_or(format!("--size: expected WxH: {value}"))?;
    Ok((number("--size", w)?, number("--size", h)?))
}

#[cfg(test)]
mod tests {
    use super::{Args, parse};
    use std::path::PathBuf;

    fn run(args: &[&str]) -> Result<Args, String> {
        parse(args.iter().map(Into::into))
    }

    #[test]
    fn no_arguments_mean_the_picker() {
        assert_eq!(run(&[]), Ok(Args::default()));
    }

    #[test]
    fn reads_the_folder_and_every_flag_in_any_order() {
        assert_eq!(
            run(&[
                "--auto",
                "20",
                "D:/a b",
                "--size",
                "1616x1080",
                "--max",
                "5",
                "--index",
                "D:/t/index.sqlite",
                "--settings",
                "D:/t/settings.sqlite",
                "--import-settings",
                "D:/t/settings.json",
                "--data-dir",
                "D:/t/data",
                "--log",
                "D:/t/Handpic.log"
            ]),
            Ok(Args {
                folder: Some(PathBuf::from("D:/a b")),
                auto: Some(20),
                size: Some((1616, 1080)),
                max: Some(5),
                index: Some(PathBuf::from("D:/t/index.sqlite")),
                settings: Some(PathBuf::from("D:/t/settings.sqlite")),
                import_settings: Some(PathBuf::from("D:/t/settings.json")),
                data_dir: Some(PathBuf::from("D:/t/data")),
                log: Some(PathBuf::from("D:/t/Handpic.log")),
            })
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(run(&["--size", "1616"]).is_err());
        assert!(run(&["--auto"]).is_err());
        assert!(run(&["--index"]).is_err());
        assert!(run(&["--settings"]).is_err());
        assert!(run(&["--import-settings"]).is_err());
        assert!(run(&["--data-dir"]).is_err());
        assert!(run(&["--log"]).is_err());
        assert!(run(&["--auto", "x"]).is_err());
        assert!(run(&["--nope"]).is_err());
        assert!(run(&["a", "b"]).is_err());
    }
}
