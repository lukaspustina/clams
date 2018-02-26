extern crate clam;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate log;
extern crate loggerv;
#[macro_use]
extern crate structopt;
extern crate subprocess;

use clam::mv_videos;
use failure::{Error, ResultExt};
use std::path::PathBuf;
use structopt::StructOpt;
use subprocess::{Exec, Redirection};

#[derive(StructOpt, Debug)]
#[structopt(name = "mv_videos",
about = "Move video files from a nested directory structure into another, flat directory",
raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct Args {
    /// File extensions to consider
    #[structopt(short = "e", long = "extension", default_value = "avi,mkv,mp4")]
    extensions: String,
    /// Only consider files bigger than this
    #[structopt(short = "s", long = "size", default_value = "100M")]
    size: String,
    /// Source directories
    #[structopt(raw(required = "true", index = "1"))]
    sources: Vec<String>,
    /// Destination directory
    #[structopt(raw(index = "2"))]
    destination: String,
    /// Only show what would be done
    #[structopt(short = "d", long = "dry")]
    dry: bool,
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u64,
}

fn run(args: Args) -> Result<(), Error> {
    let _ = mv_videos::check_size_arg(&args.size)?;
    let source_directories: Vec<_> = args.sources
        .iter()
        .map(|s| s.as_ref())
        .collect();
    let extensions = mv_videos::parse_extensions(&args.extensions)?;

    let find = mv_videos::build_find_cmd(&source_directories, &args.size, extensions.as_slice())?;
    debug!("find = {}", find);

    let res = Exec::shell(&find)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Merge)
        .capture().context(format!("Failed to spawn shell command: '{}'", find))?;
    if !res.exit_status.success() {
        return Err(format_err!("Shell command failed: '{}', because:\n{}", find, res.stdout_str()));
    }
    let files: Vec<_> = res.stdout_str()
        .lines()
        .map(|f| PathBuf::from(f))
        .collect();
    debug!("files = {:#?}", files);

    for file in &files {
        let file_name = file.file_name()
            .ok_or_else(||format_err!("Invalid file name")).context("File name from find command")?;
        println!("{:#?}", file_name);
    }

    Ok(())
}

fn main() {
    let args = Args::from_args();
    loggerv::init_with_verbosity(args.verbose)
        .unwrap_or_else(|_| panic!("Could not initialize logging"));
    debug!("args = {:#?}", args);

    match run(args) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed:");
            for c in e.causes() {
                println!("{}", c);
            }
        }
    }
}
