extern crate fern;
#[macro_use]
extern crate error_chain;
extern crate log;
extern crate indicatif;
extern crate toml;

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
extern crate tail;

pub mod config {
    use std::path::Path;

    pub trait Config {
        type ConfigStruct;

        fn from_file<T: AsRef<Path>>(file_path: T) -> ConfigResult<Self::ConfigStruct>;
    }

    error_chain! {
        types {
            ConfigError, ConfigErrorKind, ConfigResultExt, ConfigResult;
        }

        errors {
            NoSuchProfile(profile: String) {
                description("No such profile")
                display("No such profile '{}'", profile)
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
        fn read_from_file() {
            let my_config = MyConfig::from_file("examples/my_config.toml");

            assert_that(&my_config).is_ok();
        }
    }
}

pub mod console {
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
            Ok(_) => {
                if input.trim() == expected {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) => Err(Error::with_chain(e, ErrorKind::FailedToReadConfirmation)),
        }
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
    use std::fs::File;
    use std::path::Path;
    use tail;

    pub fn file_exists<T: AsRef<Path>>(path: T) -> bool {
        path.as_ref().exists()
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

    pub fn init_logging<T: Into<Output>>(out: T, default: Level, levels: Vec<ModLevel>) -> Result<()> {
        let colors = ColoredLevelConfig::new()
            .info(Color::Green)
            .debug(Color::Blue);

        let Level(default) = default;
        let mut log_levels = Dispatch::new().level(default);

        for md in levels.into_iter() {
            let ModLevel { module, level } = md;
            let Level(level) = level;
            log_levels = log_levels.level_for(module, level);
        }
        log_levels = log_levels.chain(out);

        Dispatch::new()
            .format(move |out, message, record| {
                let level = format!("{}", record.level());
                out.finish(format_args!(
                    "{}{:padding$}{}: {}",
                    colors.color(record.level()),
                    " ",
                    record.target(),
                    message,
                    padding = 6 - level.len(),
                ))
            })
            .chain(log_levels)
            .apply()
            .map_err(|e| Error::with_chain(e, ErrorKind::FailedToInitLogging))?;

        Ok(())
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

