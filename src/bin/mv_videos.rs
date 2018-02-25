extern crate clam;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate loggerv;
#[macro_use]
extern crate structopt;
extern crate subprocess;

use clam::mv_videos;
use std::path::PathBuf;
use structopt::StructOpt;
use subprocess::Exec;

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


fn run() -> Result<()> {
    let args = Args::from_args();
    loggerv::init_with_verbosity(args.verbose)?;
    debug!("args = {:#?}", args);

    let _ = mv_videos::check_size_arg(&args.size).map_err(|_| panic!("Invalid size argument"));
    let source_directories: Vec<_> = args.sources
        .iter()
        .map(|s| s.as_ref())
        .collect();
    let extensions = mv_videos::parse_extensions(&args.extensions).unwrap();

    let find = mv_videos::build_find_cmd(&source_directories, &args.size, extensions.as_slice()).unwrap();
    debug!("find = {}", find);

    let out = Exec::shell(&find)
        .capture()
        .unwrap_or_else(|_| panic!(format!("Failed to execute shell command: '{}'", find)))
        .stdout_str();
    let files: Vec<_> = out
        .lines()
        .map(|f| PathBuf::from(f))
        .collect();
    debug!("files = {:#?}", files);

    for file in &files {
        let file_name = file.file_name().unwrap();
        println!("{:#?}", file_name);
    }

    Ok(())
}

quick_main!(run);

error_chain! {
    foreign_links {
        Log(::log::SetLoggerError);
    }
}
