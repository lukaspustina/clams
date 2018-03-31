extern crate fern;
#[macro_use]
extern crate error_chain;
extern crate log;
extern crate toml;

#[cfg(test)]
#[macro_use]
extern crate clams_derive;
#[cfg(test)]
extern crate spectral;
#[cfg(test)]
extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod config {
    use std::path::Path;

    trait Config {
        type ConfigStruct;

        fn from_file<T: AsRef<Path>>(file_path: T) -> Result<Self::ConfigStruct>;
    }

    error_chain! {
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

pub mod fs {
    use std::path::Path;

    pub fn file_exists<T: AsRef<Path>>(path: T) -> bool {
        path.as_ref().exists()
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
    }
}

pub mod logging {
    use fern;
    use fern::colors::{Color, ColoredLevelConfig};
    use log;
    use std::io;

    pub fn init_logging(internal_mod: &'static str, internal_level: log::LevelFilter, default: log::LevelFilter) -> Result<()> {
        let colors = ColoredLevelConfig::new()
            .info(Color::Green)
            .debug(Color::Blue);
        fern::Dispatch::new()
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
            .chain(
                fern::Dispatch::new()
                    .level(default)
                    .level_for(internal_mod, internal_level)
                    .chain(io::stderr()),
            )
            .apply()
            .map_err(|e| Error::with_chain(e, ErrorKind::FailedToInitLogging))?;

        Ok(())
    }

    pub fn int_to_log_level(n: u64) -> log::LevelFilter {
        match n {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    }

    error_chain! {
        errors {
            FailedToInitLogging {
                description("Failed to init logging")
            }
        }
    }
}

