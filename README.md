# ffmpeg_driver

This program drives convert the input mkvs in an input directory into an AV1
and Opus mkv for watching later.

## Important notes

This will only encode the first video and audio stream.

I am not currently planning on implementing any parsing of the input stream metadata.

## Usage

```
Usage: ffmpeg_driver [OPTIONS] --preset <preset> --crf <crf> [directory]...

Arguments:
  [directory]...
          A directory to process

Options:
      --preset <preset>
          [0..13]

      --crf <crf>
          [0..63]

      --fps <fps>
          ntsc-film: 24000/1001
          ntsc: 30000/1001
          pal: 25/1
          film: 24/1

      --trial
          Perform a short trial encoding to test quality. Approximately 2 minutes in length.

  -h, --help
          Print help information (use `-h` for a summary)

  -V, --version
          Print version information
```

## Dependencies

* ffmpeg
* opusenc
* SvtAv1EncApp
* mkvextract
* mkvmerge
