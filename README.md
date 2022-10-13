# ffmpeg_driver

This program drives convert the input mkvs in an input directory into an AV1
and Opus mkv for watching later.

## Important notes

All prebuilt binaries are for Linux x86-64.

Error handling still needs some work.

Input is assumed to be telecined NTSC film. If your input files are not you
will need to change the hard coded fps conversion by ffmpeg.

## Dependencies

* ffmpeg
* opusenc
* SvtAv1EncApp
* mkvextract
* mkvmerge

