extern crate colored;
#[macro_use]
extern crate error_chain;
extern crate fern;
extern crate log;
extern crate indicatif;
extern crate tail;
extern crate toml;

mod reexports {
    #[doc(hidden)] pub use colored::*;
    #[doc(hidden)] pub use indicatif::*;
    #[doc(hidden)] pub use log::*;
}

pub mod prelude {
    pub use reexports::*;

    pub use config::{Config, default_locations};
    pub use console::ask_for_confirmation;
    pub use fs::FileExt;
    pub use logging::{Level, ModLevel, init_logging};
    pub use progress::ProgressStyleExt;
}

#[cfg(test)]
#[macro_use]
extern crate clams_derive;
#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
extern crate spectral;
#[cfg(test)]
extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod config {
    use fs::home_dir;

    use std::path::{Path, PathBuf};

    pub mod prelude {
        pub use config::{Config, ConfigError, ConfigErrorKind, ConfigResult};
    }

    pub trait Config {
        type ConfigStruct;

        fn from_file<T: AsRef<Path>>(file_path: T) -> ConfigResult<Self::ConfigStruct>;

        fn smart_load<T: AsRef<Path>>(file_paths: &[T]) -> ConfigResult<Self::ConfigStruct>;
    }

    pub fn default_locations(config_file_name: &str) -> Vec<PathBuf> {
        let mut locations: Vec<PathBuf> = Vec::new();

        if let Some(mut path) = home_dir() {
            let home_config = format!(".{}", config_file_name);
            path.push(home_config);
            locations.push(path);
        }

        let mut etc = PathBuf::new();
        etc.push("/etc");
        etc.push(config_file_name);
        locations.push(etc);

        locations
    }

    error_chain! {
        types {
            ConfigError, ConfigErrorKind, ConfigResultExt, ConfigResult;
        }

        errors {
            NoSuitableConfigFound(configs: Vec<String>) {
                description("No suitable configuration found")
                display("No suitable configuration found '{:?}'", configs)
            }
        }

        foreign_links {
            CouldNotRead(::std::io::Error);
            CouldNotParse(::toml::de::Error);
        }
    }

    #[cfg(test)]
    mod test {
        pub use super::*;
        pub use spectral::prelude::*;

        #[derive(Config, Debug, Default, Serialize, Deserialize, PartialEq)]
        struct MyConfig {
            pub general: General,
        }

        #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
        struct General {
            pub name: String,
        }

        #[test]
        fn from_file_okay() {
            let my_config = MyConfig::from_file("examples/my_config.toml");

            assert_that(&my_config).is_ok();
        }

        #[test]
        fn smart_load_okay() {
            let locations = vec!["tmp/my_config.toml", "tmp2/my_config.toml", "examples/my_config.toml"];

            let res = MyConfig::smart_load(&locations);

            assert_that(&res).is_ok();
        }

        #[test]
        fn smart_load_faild() {
            let locations = vec!["tmp/my_config.toml", "tmp2/my_config.toml"];

            let res = MyConfig::smart_load(&locations);

            assert_that(&res).is_err();
        }

        #[test]
        fn default_locations_okay() {
            let home_dir = home_dir().expect("Could not retrieve username");
            let mut home_config = PathBuf::from(home_dir);
            home_config.push(".my_config.toml");
            let expected: Vec<PathBuf> = vec![
                home_config,
                PathBuf::from("/etc/my_config.toml"),
            ];

            let res = default_locations("my_config.toml");

            assert_that(&res).is_equal_to(expected);
        }

        #[test]
        fn smart_load_from_default_locations_and_local() {
            let mut locations = default_locations("my_config.toml");
            locations.push(PathBuf::from("examples/my_config.toml"));

            let res = MyConfig::smart_load(&locations);

            assert_that(&res).is_ok();
        }
    }
}

pub mod console {
    use colored;
    use std::io::{self, BufRead, BufReader, Write};

    pub fn ask_for_confirmation(prompt: &str, expected: &str) -> Result<bool> {
        let mut reader = BufReader::new(io::stdin());
        let mut writer = io::stdout();
        ask_for_confirmation_from(&mut reader, &mut writer, prompt, expected)
    }

    pub fn ask_for_confirmation_from<R: BufRead, W: Write>(reader: &mut R, writer: &mut W, prompt: &str, expected: &str) -> Result<bool> {
        let question = format!("{}", prompt);
        writer.write(question.as_bytes())
            .chain_err(|| ErrorKind::FailedToReadConfirmation)?;
        writer.flush()
            .chain_err(|| ErrorKind::FailedToReadConfirmation)?;

        let mut input = String::new();
        match reader.read_line(&mut input) {
            Ok(_) => Ok(input.trim() == expected),
            Err(e) => Err(Error::with_chain(e, ErrorKind::FailedToReadConfirmation)),
        }
    }

    pub fn set_color_off() -> () {
        set_color(false);
    }

    pub fn set_color(on: bool) -> () {
        colored::control::set_override(on); 
    }

    error_chain! {
        errors {
            FailedToReadConfirmation {
                description("Failed to read confirmation")
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        use quickcheck::{quickcheck, TestResult};
        use spectral::prelude::*;
        use std::io::BufWriter;

        #[test]
        fn ask_for_yes_from_okay() {
            let answer = "yes".to_owned();
            let mut input = BufReader::new(answer.as_bytes());
            let output_buf = Vec::new();
            let mut output = BufWriter::new(output_buf);

            let res = ask_for_confirmation_from(&mut input, &mut output, "This is just a test prompt: ", "yes");

            assert_that(&res).is_ok().is_true();
        }

        #[test]
        fn ask_for_yes_reader_quick() {
            fn prop(x: String) -> TestResult {
                let expected = "yes";

                if x.len() > 3 || x == expected {
                    return TestResult::discard();
                }

                let mut input = BufReader::new(x.as_bytes());
                let output_buf = Vec::new();
                let mut output = BufWriter::new(output_buf);

                let res = ask_for_confirmation_from(&mut input, &mut output, "This is just a test prompt: ", expected)
                    .unwrap();

                TestResult::from_bool(res == false)
            }

            quickcheck(prop as fn(String) -> TestResult);
        }
    }
}

pub mod fs {
    use std::io::{BufReader, BufWriter};
    use std::env;
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use tail;

    pub fn file_exists<T: AsRef<Path>>(path: T) -> bool {
        path.as_ref().exists()
    }

    pub fn home_dir() -> Option<PathBuf> {
        env::home_dir()
    }

    pub trait FileExt {
        fn read_last_line(self) -> ::std::io::Result<String>;
    }

    impl FileExt for File {
        fn read_last_line(self) -> ::std::io::Result<String> {
            let mut fd = BufReader::new(self);
            let mut reader = tail::BackwardsReader::new(10, &mut fd);
            let mut buffer = String::new();
            {
                let mut writer = BufWriter::new(
                    unsafe {
                        buffer.as_mut_vec()
                    }
                );
                reader.read_all(&mut writer);
            }
            let line = buffer.lines().last().map(|s| s.to_owned()).unwrap_or_else(|| String::new());
            Ok(line)
        }
    }

    #[cfg(test)]
    mod test {
        pub use super::*;
        pub use spectral::prelude::*;

        mod file_exists {
            use super::*;

            #[test]
            fn no_such_file() {
                let file_name = "no_such.file";
                let res = file_exists(&file_name);
                assert_that(&res).is_false();
            }

            #[test]
            fn file_does_exists() {
                let file_name = "tests/data/file.exists";
                let res = file_exists(&file_name);
                assert_that(&res).is_true();
            }
        }

        mod file_ext {
            use super::*;

            #[test]
            fn read_last_line_okay() {
                let file = File::open("tests/data/tail.txt").expect("Could not open tail.txt");

                let last_line = file.read_last_line().expect("Could not read last line");

                assert_that(&last_line).is_equal_to("-- Marcus Marcus Aurelius".to_owned());
            }
        }
    }
}

pub mod logging {
    use fern::{Dispatch, Output};
    use fern::colors::{Color, ColoredLevelConfig};
    use log;

    #[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
    pub struct Level(pub log::LevelFilter);

    impl From<u64> for Level {
        fn from(level: u64) -> Self {
            match level {
                0 => Level(log::LevelFilter::Warn),
                1 => Level(log::LevelFilter::Info),
                2 => Level(log::LevelFilter::Debug),
                _ => Level(log::LevelFilter::Trace),
            }
        }
    }

    #[derive(Debug)]
    pub struct ModLevel {
        pub module: String,
        pub level: Level,
    }

    #[derive(Debug)]
    pub struct LogConfig {
        out: Output,
        color: bool,
        default: Level,
        levels: Vec<ModLevel>,
        context: Option<String>,
    }

    pub fn init_logging(log_config: LogConfig) -> Result<()> {
        let Level(default) = log_config.default;
        let mut log_levels = Dispatch::new().level(default);

        for md in log_config.levels.into_iter() {
            let ModLevel { module, level } = md;
            let Level(level) = level;
            log_levels = log_levels.level_for(module, level);
        }
        log_levels = log_levels.chain(log_config.out);

        let format = if log_config.color {
            format_with_color(log_config.context)
        } else {
            format_no_color(log_config.context)
        };
        format
            .chain(log_levels)
            .apply()
            .map_err(|e| Error::with_chain(e, ErrorKind::FailedToInitLogging))?;

        Ok(())
    }

    fn format_with_color(context: Option<String>) -> Dispatch {
        let colors = ColoredLevelConfig::new()
            .info(Color::Green)
            .debug(Color::Blue);
        let context = if let Some(c) = context {
            format!("Context={}", c)
        } else {
            "".to_owned()
        };
        Dispatch::new()
            .format(move |out, message, record| {
                let level = format!("{}", record.level());
                out.finish(format_args!(
                    "{}{}{:padding$}{}: {}",
                    context,
                    colors.color(record.level()),
                    " ",
                    record.target(),
                    message,
                    padding = 6 - level.len(),
                ))
            })
    }

    fn format_no_color(context: Option<String>) -> Dispatch {
        let context = if let Some(c) = context {
            format!("Context={}", c)
        } else {
            "".to_owned()
        };
        Dispatch::new()
            .format(move |out, message, record| {
                let level = format!("{}", record.level());
                out.finish(format_args!(
                    "{}{}{:padding$}{}: {}",
                    context,
                    record.level(),
                    " ",
                    record.target(),
                    message,
                    padding = 6 - level.len(),
                ))
            })
    }

    error_chain! {
        errors {
            FailedToInitLogging {
                description("Failed to init logging")
            }
        }
    }
}

pub mod progress {
    use indicatif::ProgressStyle;

    pub trait ProgressStyleExt {
        fn default_clams_spinner() -> ProgressStyle;

        fn default_clams_bar() -> ProgressStyle;
    }

    impl ProgressStyleExt for ProgressStyle {
        fn default_clams_spinner() -> ProgressStyle {
            ProgressStyle::default_spinner()
                .template("{prefix:.bold.dim} [{elapsed}] {spinner} {wide_msg}")
        }

        fn default_clams_bar() -> ProgressStyle {
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:20.blue/blue}] {pos}/{len} ({eta}) {wide_msg} {spinner:.blue}")
        }

    }
}

