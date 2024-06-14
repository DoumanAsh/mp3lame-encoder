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
    let mut mp3_encoder = Builder::new().expect("Create LAME builder");
    mp3_encoder.set_num_channels(spec_channels as u8).expect("set channels");
    mp3_encoder.set_sample_rate(spec.rate).expect("set sample rate");
    mp3_encoder.set_brate(mp3lame_encoder::Bitrate::Kbps192).expect("set brate");
    mp3_encoder.set_quality(mp3lame_encoder::Quality::Best).expect("set quality");
    mp3_encoder.set_id3_tag(Id3Tag {
        title: b"Bell",
        artist: &[],
        album: b"Test",
        album_art: ALBUM_ART,
        year: b"2022",
        comment: b"Just some test shit",
    }).expect("success");
    let mut mp3_encoder = mp3_encoder.build().expect("To initialize LAME encoder");

    mp3_out_buffer.reserve(MAX_ALBUM_ART_SIZE);

    let mut samples_num = audio_buf.frames();
    let encoded_size = match audio_buf {
        AudioBufferRef::F32(audio_buf) => {
            let planes = audio_buf.planes();
            let planes = planes.planes();
            assert_eq!(planes.len(), 1);
            let input = MonoPcm(planes[0]);
            assert_eq!(samples_num, input.0.len());
            mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
            mp3_encoder.encode(input, mp3_out_buffer.spare_capacity_mut()).expect("To encode")
        }
        AudioBufferRef::F64(audio_buf) => {
            let planes = audio_buf.planes();
            let planes = planes.planes();
            assert_eq!(planes.len(), 1);
            let input = MonoPcm(planes[0]);
            assert_eq!(samples_num, input.0.len());
            mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
            mp3_encoder.encode(input, mp3_out_buffer.spare_capacity_mut()).expect("To encode")
        }
        _ => panic!("Unexpected"),
    };

    unsafe {
        mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
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

        let encoded_size = match audio_buf {
            AudioBufferRef::F32(audio_buf) => {
                let planes = audio_buf.planes();
                let planes = planes.planes();
                assert_eq!(planes.len(), 1);
                let input = MonoPcm(planes[0]);
                assert_eq!(samples_num, input.0.len());
                mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
                mp3_encoder.encode(input, mp3_out_buffer.spare_capacity_mut()).expect("To encode")
            }
            AudioBufferRef::F64(audio_buf) => {
                let planes = audio_buf.planes();
                let planes = planes.planes();
                assert_eq!(planes.len(), 2);
                let input = MonoPcm(planes[0]);
                assert_eq!(samples_num, input.0.len());
                mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_num));
                mp3_encoder.encode(input, mp3_out_buffer.spare_capacity_mut()).expect("To encode")
            }
            _ => panic!("Unexpected"),
        };


        unsafe {
            mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
        }
    }

    let encoded_size = mp3_encoder.flush::<FlushNoGap>(mp3_out_buffer.spare_capacity_mut()).expect("to flush");
    unsafe {
        mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
    }
    fs::write(NEW_FILE, &mp3_out_buffer).expect("write file")
}
