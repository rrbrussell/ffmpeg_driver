# ffmpeg_driver

This program drives convert the input mkvs in an input directory into an AV1
and Opus mkv for watching later.

## Important notes

Error handling still needs some work.

Input is assumed to be telecined NTSC film. If your input files are not you
will need to change the hard coded fps conversion by ffmpeg.

## Freespace requirements

Quite a lot. 2 pass runs with SvtAvcEncApp are difficult on a pipe so I dump
the input video stream into a 10Bit YUV420p temporary file. Figure on about
1.5GB of free space per minute for an NTSC film input. That jumps to 8.5GB of
free space per minute for a 24000/1001 fps 1080p input.
## Dependencies

* ffmpeg
* opusenc
* SvtAv1EncApp
* mkvextract
* mkvmerge

