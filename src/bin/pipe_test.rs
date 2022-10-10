use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

fn main() -> ExitCode {
    let cli_arguments: Vec<String> = env::args().collect();

    #[cfg(debug_assertions)]
    {
        println!("The provided arguments were:");
        dbg!(cli_arguments.clone());
    }

    if cli_arguments.len() == 1 {
        println!("Usage: ffmpeg_driver <directory>");
        return ExitCode::FAILURE;
    }

    let mut paths: Vec<PathBuf> = Vec::with_capacity(cli_arguments.len());
    for path in cli_arguments {
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
        let pb_ivf = item.with_extension("ivf");
        let temp_ivf = pb_ivf.to_str().unwrap();
        let pb_stats = item.with_extension("stats");
        let temp_stats = pb_stats.to_str().unwrap();

        let clean_message: &str = "Cleaning up and skipping to the next input file.";

        let ffmpeg_global_arguments = ["-hide_banner", "-loglevel", "fatal", "-y", "-nostats"];

        println!("Extracting video.");
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
            "-r",
            "24000/1001",
            "-frames", //Just pull out 1 minute of data.
            "1440",
            "-",
        ];
        let mut ffmpeg_y4m = Command::new("ffmpeg")
            .args(ffmpeg_global_arguments)
            .args(ffmpeg_y4m_arguments)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Cannot create a y4m file");
        println!("Encoding video with SvtAv1EncApp");
        let av1_encoder_pass_1_arguments = [
            "-i",
            "stdin",
            "--preset",
            "5",
            "--lookahead",
            "120",
            "--progress",
            "2",
            "--scd",
            "1",
            "-b",
            temp_ivf,
            "--crf",
            "38",
            "--pass",
            "1",
            "--stats",
            temp_stats,
        ];
        let av1_encoder_pass_1 = Command::new("SvtAv1EncApp")
            .args(av1_encoder_pass_1_arguments)
            .stdin(ffmpeg_y4m.stdout.take().unwrap())
            .status()
            .expect("Cannot start SvtAv1EncApp.");
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
            "-r",
            "24000/1001",
            "-frames", //Just pull out 1 minute of data.
            "1440",
            "-",
        ];
        let mut ffmpeg_y4m = Command::new("ffmpeg")
            .args(ffmpeg_global_arguments)
            .args(ffmpeg_y4m_arguments)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Cannot create a y4m file");
        println!("Encoding video with SvtAv1EncApp");
        let av1_encoder_pass_2_arguments = [
            "-i",
            "stdin",
            "--preset",
            "5",
            "--lookahead",
            "120",
            "--progress",
            "2",
            "--scd",
            "1",
            "-b",
            temp_ivf,
            "--crf",
            "38",
            "--pass",
            "2",
            "--stats",
            temp_stats,
        ];
        let av1_encoder_pass_2 = Command::new("SvtAv1EncApp")
            .args(av1_encoder_pass_2_arguments)
            .stdin(ffmpeg_y4m.stdout.take().unwrap())
            .status()
            .expect("Cannot start SvtAv1EncApp.");
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
    }
    return ExitCode::SUCCESS;
}
