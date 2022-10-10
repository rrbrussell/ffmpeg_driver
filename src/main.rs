use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

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
        let pb_wav = item.with_extension("wav");
        let temp_wav = pb_wav.to_str().unwrap();
        let pb_chapters = item.with_extension("xml");
        let temp_chapters = pb_chapters.to_str().unwrap();
        let pb_y4m = item.with_extension("y4m");
        let temp_y4m = pb_y4m.to_str().unwrap();
        let pb_opus = item.with_extension("opus");
        let temp_opus = pb_opus.to_str().unwrap();
        let pb_ivf = item.with_extension("ivf");
        let temp_ivf = pb_ivf.to_str().unwrap();
        let pb_output = item.with_extension("out").with_extension("mkv");
        let output_mkv = pb_output.to_str().unwrap();

        let ffmpeg_global_arguments = ["-hide_banner", "-loglevel", "fatal", "-y", "-stats"];
        println!("Processing {} into {}", input_mkv, output_mkv);
        println!("Extracting audio.");
        let ffmpeg_wav_arguments = [
            "-i",
            input_mkv,
            "-map",
            "0:a:0",
            "-acodec",
            "pcm_s24le",
            temp_wav,
        ];
        let mut ffmpeg_wav = Command::new("ffmpeg")
            .args(ffmpeg_global_arguments)
            .args(ffmpeg_wav_arguments)
            .spawn()
            .expect("Cannot create a wav file");
        match ffmpeg_wav.wait() {
            Err(e) => {
                println!("ffmpeg failed with {e}");
                return ExitCode::FAILURE;
            }
            Ok(res) => {
                if !res.success() {
                    println!("Extracting audio failed.");
                    _ = fs::remove_file(pb_wav);
                    return ExitCode::FAILURE;
                }
            }
        }
        println!("Audio extraction complete.");
        println!("Encoding audio with opus.");
        let opusenc_arguments = ["--bitrate", "192", "--vbr", temp_wav, temp_opus];
        let mut opusenc = Command::new("opusenc")
            .args(opusenc_arguments)
            .spawn()
            .expect("Cannot encode opus file.");
        if let Err(e) = opusenc.wait() {
            println!("opusenc failed with {e}");
            return ExitCode::FAILURE;
        }
        // Throw away any errors. One unix platforms if you can create a file you can remove a file.
        // We created the file earlier.
        _ = fs::remove_file(pb_wav);

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
            temp_y4m,
        ];
        let mut ffmpeg_y4m = Command::new("ffmpeg")
            .args(ffmpeg_global_arguments)
            .args(ffmpeg_y4m_arguments)
            .spawn()
            .expect("Cannot create a y4m file");
        if let Err(e) = ffmpeg_y4m.wait() {
            println!("ffmpeg failed with {e}");
            return ExitCode::FAILURE;
        }
        println!("Video extraction complete");
        println!("Encoding video with SvtAv1EncApp");
        let av1_encoder_arguments = [
            "-i",
            temp_y4m,
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
            "--passes",
            "2",
        ];
        let mut av1_encoder = Command::new("SvtAv1EncApp")
            .args(av1_encoder_arguments)
            .spawn()
            .expect("Cannot start SvtAv1EncApp.");
        if let Err(e) = av1_encoder.wait() {
            println!("SvtAv1EncApp failed with {e}");
            return ExitCode::FAILURE;
        }
        println!("Video encoding complete.");
        _ = fs::remove_file(pb_y4m);

        println!("Extracting the chapters.");
        let mkvextract_chapters_arguments = [input_mkv, "chapters", temp_chapters];
        let mut mkvextract_chapters = Command::new("mkvextract")
            .args(mkvextract_chapters_arguments)
            .spawn()
            .expect("Cannot create a chapters file.");
        if let Err(e) = mkvextract_chapters.wait() {
            println!("mkvextract failed with {e}");
            return ExitCode::FAILURE;
        }
        println!("Creating the output file.");
        let mkvmerge_arguments = [
            temp_ivf,
            temp_opus,
            "--chapters",
            temp_chapters,
            "-o",
            output_mkv,
        ];
        let mut mkvmerge = Command::new("mkvmerge")
            .args(mkvmerge_arguments)
            .spawn()
            .expect("Cannot start mkvmerge.");
        if let Err(e) = mkvmerge.wait() {
            println!("mkvmerge failed with {e}");
        }
        println!("Completed converting {input_mkv} to {output_mkv}");
        _ = fs::remove_file(pb_ivf);
        _ = fs::remove_file(pb_opus);
        _ = fs::remove_file(pb_chapters);
    }
    return ExitCode::SUCCESS;
}
