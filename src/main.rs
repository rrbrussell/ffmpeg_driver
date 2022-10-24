use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use clap::command;
use clap::Arg;
use clap::ArgAction;

fn main() -> ExitCode {
    let commands: clap::Command = build_argument_processor();
    let arguments = commands.get_matches();

    let preset: [&str; 2] = [
        "--preset",
        &arguments
            .get_one::<u8>("preset")
            .expect("Clap screwed up processing the arguments")
            .to_string(),
    ];

    let crf: [&str; 2] = [
        "--crf",
        &arguments
            .get_one::<u8>("crf")
            .expect("Clap screwed up processing the arguments.")
            .to_string(),
    ];

    let trial: [&str; 2];
    if arguments.get_flag("trial") {
        trial = ["-t", "120"];
    } else {
        trial = ["", ""];
    }

    let framerate: [&str; 2];
    match arguments.get_one::<String>("fps") {
        None => framerate = ["", ""],
        Some(value) => {
            if value.starts_with("ntsc") {
                if value.ends_with("-film") {
                    framerate = ["-r", "24000/1001"];
                } else {
                    framerate = ["-r", "30000/1001"];
                }
            } else {
                if value.eq("pal") {
                    framerate = ["-r", "25/1"];
                } else {
                    framerate = ["-r", "24/1"];
                }
            }
        }
    }

    let mut paths: Vec<PathBuf> = Vec::with_capacity(env::args().len());
    for path in arguments
        .get_many::<String>("directory")
        .expect("Clap screwed up processing the arguments.")
    {
        match PathBuf::from(path.as_str()).canonicalize() {
            Result::Err(e) => {
                eprintln!("I cannot process: {} because {}", path, e);
            }
            Result::Ok(pb) => {
                if !paths.contains(&pb) {
                    #[cfg(debug_assertions)]
                    {
                        println!("Added {:?} to list to process.", pb);
                    }
                    paths.push(pb);
                } else {
                    #[cfg(debug_assertions)]
                    {
                        println!("Skipping {:?} because we already have it on the list.", pb);
                    }
                }
            }
        }
    }

    let mut i: usize = 0;
    while i < paths.len() {
        if !paths[i].is_dir() {
            if cfg!(debug_assertions) {
                println!(
                    "Removing: {:?}. It is not a directory.",
                    paths.swap_remove(i)
                );
            } else {
                _ = paths.swap_remove(i);
            }
        } else {
            i += 1;
        }
    }

    let mut to_proccess: Vec<PathBuf> = Vec::with_capacity(10);
    for directory in paths {
        match directory.read_dir() {
            Err(e) => {
                eprintln!("Error processing {:?}: {e}", directory);
            }
            Ok(iter) => {
                for item in iter {
                    match item {
                        Err(e) => eprintln!("{e}"),
                        Ok(good) => {
                            let pb = good.path();
                            let ext = pb.extension();
                            if ext.is_some() {
                                let ext = ext.unwrap();
                                if ext == "mkv" {
                                    to_proccess.push(pb);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("We are going to process the following files.");
    for item in to_proccess.iter() {
        println!("{}", item.display());
    }

    println!("Processing");

    for item in to_proccess.iter() {
        let input_mkv = item.to_str().unwrap();
        let pb_chapters = item.with_extension("xml");
        let temp_chapters = pb_chapters.to_str().unwrap();
        let pb_opus = item.with_extension("opus");
        let temp_opus = pb_opus.to_str().unwrap();
        let pb_ivf = item.with_extension("ivf");
        let temp_ivf = pb_ivf.to_str().unwrap();
        let mut output_name = String::from(item.file_stem().unwrap().to_str().unwrap());
        output_name.push_str("-out.mkv");
        let pb_output = item.with_file_name(output_name);
        let output_mkv = pb_output.to_str().unwrap();
        let pb_stats = item.with_extension("stats");
        let temp_stats = pb_stats.to_str().unwrap();

        let fix_message: &str = "Fix the problematic input file.";
        let clean_message: &str = "Cleaning up and skipping to the next input file.";

        let ffmpeg_global_arguments = ["-hide_banner", "-loglevel", "fatal", "-y", "-nostats"];
        println!("Processing {} into {}", input_mkv, output_mkv);
        println!("Encoding the audio.");
        let ffmpeg_wav_arguments = [
            "-i",
            input_mkv,
            "-map",
            "0:a:0",
            "-acodec",
            "pcm_s24le",
            "-f",
            "wav",
            "-",
        ];
        let mut ffmpeg_wav = Command::new("ffmpeg")
            .args(ffmpeg_global_arguments)
            .args(trial)
            .args(ffmpeg_wav_arguments)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Cannot find ffmpeg.");
        let opusenc_arguments = ["--bitrate", "192", "--vbr", "-", temp_opus];
        let opusenc = Command::new("opusenc")
            .args(opusenc_arguments)
            .stdin(ffmpeg_wav.stdout.take().unwrap())
            .status()
            .expect("Cannot find opusenc.");
        if !opusenc.success() {
            println!("{clean_message}");
            _ = fs::remove_file(pb_opus);
            _ = ffmpeg_wav.wait();
            break;
        } else {
            _ = ffmpeg_wav.wait();
        }

        println!("Encoding video.");
        let ffmpeg_y4m_arguments = [
            "-i",
            input_mkv,
            "-map",
            "0:v:0",
            "-pix_fmt",
            "yuv420p10le",
            "-f",
            "yuv4mpegpipe",
            "-strict",
            "-1",
            "-",
        ];
        let mut ffmpeg_y4m = Command::new("ffmpeg")
            .args(ffmpeg_global_arguments)
            .args(trial)
            .args(ffmpeg_y4m_arguments)
            .args(framerate)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Cannot find ffmpeg.");
        let av1_encoder_pass_1_arguments = [
            "-i",
            "stdin",
            "--lookahead",
            "120",
            "--progress",
            "2",
            "--scd",
            "1",
            "-b",
            temp_ivf,
            "--pass",
            "1",
            "--stats",
            temp_stats,
        ];
        let av1_encoder_pass_1 = Command::new("SvtAv1EncApp")
            .args(av1_encoder_pass_1_arguments)
            .args(preset)
            .args(crf)
            .stdin(ffmpeg_y4m.stdout.take().unwrap())
            .status()
            .expect("Cannot find SvtAv1EncApp.");
        if !av1_encoder_pass_1.success() {
            println!("{clean_message}");
            _ = fs::remove_file(pb_ivf);
            _ = fs::remove_file(pb_stats);
            _ = ffmpeg_y4m.wait();
            break;
        } else {
            println!("Pass 1 Completed.");
            _ = fs::remove_file(pb_ivf.clone());
            _ = ffmpeg_y4m.wait();
        }

        let mut ffmpeg_y4m = Command::new("ffmpeg")
            .args(ffmpeg_global_arguments)
            .args(trial)
            .args(ffmpeg_y4m_arguments)
            .args(framerate)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Cannot find ffmpeg.");
        println!("Starting Pass 2.");
        let av1_encoder_pass_2_arguments = [
            "-i",
            "stdin",
            "--lookahead",
            "120",
            "--progress",
            "2",
            "--scd",
            "1",
            "-b",
            temp_ivf,
            "--pass",
            "2",
            "--stats",
            temp_stats,
        ];
        let av1_encoder_pass_2 = Command::new("SvtAv1EncApp")
            .args(av1_encoder_pass_2_arguments)
            .args(preset)
            .args(crf)
            .stdin(ffmpeg_y4m.stdout.take().unwrap())
            .status()
            .expect("Cannot find SvtAv1EncApp.");
        if !av1_encoder_pass_2.success() {
            println!("{clean_message}");
            _ = fs::remove_file(pb_ivf);
            _ = fs::remove_file(pb_stats);
            _ = ffmpeg_y4m.wait();
            break;
        } else {
            println!("Pass 2 Completed.");
            _ = ffmpeg_y4m.wait();
            _ = fs::remove_file(pb_stats);
        }

        println!("Extracting the chapters.");
        let chapters_in_input: bool;
        let mkvextract_chapters_arguments = [input_mkv, "chapters", temp_chapters];
        let mut mkvextract_chapters = Command::new("mkvextract")
            .args(mkvextract_chapters_arguments)
            .spawn()
            .expect("Cannot create a chapters file.");
        match mkvextract_chapters.wait() {
            Err(_) => {
                println!("{clean_message}");
                _ = fs::remove_file(pb_chapters);
                _ = fs::remove_file(pb_ivf);
                _ = fs::remove_file(pb_opus);
                break;
            }
            Ok(_) => {
                if pb_chapters.is_file() {
                    chapters_in_input = true;
                } else {
                    chapters_in_input = false;
                }
            }
        }

        println!("Creating the output file.");
        let mkvmerge_arguments = [temp_ivf, temp_opus, "-o", output_mkv];
        let mkvmerge_chapters = ["--chapters", temp_chapters];
        let mut mkvmerge_partial = Command::new("mkvmerge");
        mkvmerge_partial.args(mkvmerge_arguments);
        if chapters_in_input {
            mkvmerge_partial.args(mkvmerge_chapters);
        }
        let mut mkvmerge = mkvmerge_partial.spawn().expect("Cannot start mkvmerge.");
        if !mkvmerge.wait().expect(fix_message).success() {
            println!("{clean_message}");
            _ = fs::remove_file(pb_output);
            _ = fs::remove_file(pb_ivf);
            _ = fs::remove_file(pb_opus);
            _ = fs::remove_file(pb_chapters);
            break;
        } else {
            println!("Completed converting {input_mkv} to {output_mkv}");
            _ = fs::remove_file(pb_ivf);
            _ = fs::remove_file(pb_opus);
            _ = fs::remove_file(pb_chapters);
        }
    }
    return ExitCode::SUCCESS;
}

fn build_argument_processor() -> clap::Command {
    let mut temp: clap::Command = command!();
    temp = temp.arg(
        Arg::new("preset")
            .long("preset")
            .value_parser(clap::value_parser!(u8).range(0..14))
            .action(ArgAction::Set)
            .required(true)
            .help("[0..13]"),
    );
    temp = temp.arg(
        Arg::new("crf")
            .long("crf")
            .value_parser(clap::value_parser!(u8).range(0..64))
            .action(ArgAction::Set)
            .required(true)
            .help("[0..63]"),
    );
    temp = temp.arg(
        Arg::new("fps")
            .long("fps")
            .value_parser(["ntsc-film", "ntsc", "pal", "film"])
            .action(ArgAction::Set)
            .required(false)
            .next_line_help(true)
            .hide_possible_values(true)
            .help("[possible values: ntsc-film, ntsc, pal, film]")
            .long_help("ntsc-film: 24000/1001\nntsc: 30000/1001\npal: 25/1\nfilm: 24/1"),
    );
    temp = temp.arg(Arg::new("trial").long("trial").action(ArgAction::SetTrue).required(false).help(
        "Perform a short trial encoding to test quality. Approximately 2 minutes in length.",
    ));
    temp = temp.arg(
        Arg::new("directory")
            .action(ArgAction::Append)
            .required(false)
            .value_parser(clap::builder::NonEmptyStringValueParser::new())
            .help("A directory to process"),
    );
    return temp;
}
