use std::{fs, io};

use symphonia::core::audio::AudioBufferRef;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::core::errors::Error as SymError;

use mp3lame_encoder::{Builder, MonoPcm, FlushNoGap, Id3Tag, MAX_ALBUM_ART_SIZE};

static ALBUM_ART: &[u8] = include_bytes!("album_art.jpg");

#[test]
fn should_decode_and_encode() {
    const FILE: &str = "tests/Bell3.ogg";
    const NEW_FILE: &str = "tests/Bell3_encoded.mp3";

    let file = fs::File::open(FILE).expect("open FILE");
    let file = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("ogg");

    let format_opts = Default::default();
    let metadata_opts = Default::default();
    let decoder_opts = Default::default();

    // Probe the media source stream for a format.
    let probed = symphonia::default::get_probe().format(&hint, file, &format_opts, &metadata_opts).expect("To probe mp3 file");
    // Get the format reader yielded by the probe operation.
    let mut format = probed.format;
    let track = format.default_track().expect("Get default track");
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts).unwrap();

    // Store the track identifier, we'll use it to filter packets.
    let track_id = track.id;

    let first_packet = loop {
        let packet = format.next_packet().expect("to get packet");
        if packet.track_id() != track_id {
            continue
        }
        break packet;
    };

    let audio_buf = decoder.decode(&first_packet).expect("To decode first packet");
    let spec = *audio_buf.spec();
    let spec_channels = spec.channels.count();

    let mut mp3_out_buffer = Vec::new();

    // Build the encoder using builder-like ernomonics
    let mut mp3_encoder = Builder::new().expect("Create LAME builder")
        .with_num_channels(spec_channels as u8).expect("set channels")
        .with_sample_rate(spec.rate).expect("set sample rate")
        .with_brate(mp3lame_encoder::Birtate::Kbps192).expect("set brate")
        .with_quality(mp3lame_encoder::Quality::Best).expect("set quality")
        .with_vbr_mode(mp3lame_encoder::VbrMode::Mtrh).expect("set VBR")
        .with_vbr_quality(mp3lame_encoder::Quality::Best).expect("set VBR quality")
        .with_to_write_vbr_tag(true).expect("set to write VBR tag")
        .with_id3_tag(Id3Tag {
            title: b"Bell",
            artist: &[],
            album: b"Test",
            album_art: ALBUM_ART,
            year: b"2022",
            comment: b"Just some test shit",
        }).expect("Id3 tag")
        .build().expect("To initialize LAME encoder");

    mp3_out_buffer.reserve(MAX_ALBUM_ART_SIZE);

    let mut samples_num = audio_buf.frames();
    match audio_buf {
        AudioBufferRef::F32(audio_buf) => {
            let planes = audio_buf.planes();
            let planes = planes.planes();
            assert_eq!(planes.len(), 1);
            let input = MonoPcm(planes[0]);
            assert_eq!(samples_num, input.0.len());
            mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
            mp3_encoder.encode_to_vec(input, &mut mp3_out_buffer).expect("To encode");
        }
        AudioBufferRef::F64(audio_buf) => {
            let planes = audio_buf.planes();
            let planes = planes.planes();
            assert_eq!(planes.len(), 1);
            let input = MonoPcm(planes[0]);
            assert_eq!(samples_num, input.0.len());
            mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
            mp3_encoder.encode_to_vec(input, &mut mp3_out_buffer).expect("To encode");
        }
        _ => panic!("Unexpected"),
    }

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymError::IoError(error)) => match error.kind() {
                io::ErrorKind::UnexpectedEof => break,
                _ => panic!("Unexpected IO error: {error}"),
            },
            Err(error) => panic!("Unexpected error reading packets: {error}"),
        };

        if packet.track_id() != track_id {
            continue
        }

        let audio_buf = decoder.decode(&packet).expect("To decode first packet");

        samples_num = audio_buf.frames();

        match audio_buf {
            AudioBufferRef::F32(audio_buf) => {
                let planes = audio_buf.planes();
                let planes = planes.planes();
                assert_eq!(planes.len(), 1);
                let input = MonoPcm(planes[0]);
                assert_eq!(samples_num, input.0.len());
                mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
                mp3_encoder.encode_to_vec(input, &mut mp3_out_buffer).expect("To encode");
            }
            AudioBufferRef::F64(audio_buf) => {
                let planes = audio_buf.planes();
                let planes = planes.planes();
                assert_eq!(planes.len(), 2);
                let input = MonoPcm(planes[0]);
                assert_eq!(samples_num, input.0.len());
                mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
                mp3_encoder.encode_to_vec(input, &mut mp3_out_buffer).expect("To encode");
            }
            _ => panic!("Unexpected"),
        }
    }

    let mut lame_tag = Vec::new();
    assert!(mp3_encoder.is_lame_tag_written());
    assert_eq!(mp3_encoder.lame_tag_size(), 417);
    assert_eq!(mp3_encoder.lame_tag_encode_to_vec(&mut lame_tag), None);
    lame_tag.reserve(mp3_encoder.lame_tag_size());
    assert_eq!(mp3_encoder.lame_tag_encode_to_vec(&mut lame_tag).map(|len| len.get()), Some(417));

    let _ = mp3_encoder.flush_to_vec::<FlushNoGap>(&mut mp3_out_buffer).expect("to flush");
    assert_eq!(mp3_encoder.id3v2_tag_size(), 94507);

    let mut output_file = fs::File::create(NEW_FILE).expect("create file");
    //Write Id3Tag first (if any)
    io::Write::write_all(&mut output_file, &mp3_out_buffer[..mp3_encoder.id3v2_tag_size()]).expect("write id3v2_tag");
    //Write Lame Tag (only after finishing encoding)
    io::Write::write_all(&mut output_file, &lame_tag).expect("write lame tag");
    //Write actual encoded mp3
    io::Write::write_all(&mut output_file, &mp3_out_buffer[mp3_encoder.id3v2_tag_size()..]).expect("write encoded mp3");
    io::Write::flush(&mut output_file).expect("flush mp3");
}

#[test]
fn should_verify_vbr_tag_is_not_present_without_encode() {
    let mp3_encoder = Builder::new().expect("Create LAME builder").with_vbr_mode(mp3lame_encoder::VbrMode::Off).expect("set vbr").build().expect("finish build");
    assert_eq!(mp3_encoder.lame_tag_size(), 0);
    let mp3_encoder = Builder::new().expect("Create LAME builder").with_vbr_mode(mp3lame_encoder::VbrMode::Mtrh).expect("set vbr")
                                                                  .with_vbr_quality(mp3lame_encoder::Quality::Best).expect("set quality")
                                                                  .build().expect("finish build");
    assert!(mp3_encoder.is_lame_tag_written());
    assert_eq!(mp3_encoder.lame_tag_size(), 0);
}
