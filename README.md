# mp3lame-encoder

[![Rust](https://github.com/DoumanAsh/mp3lame-encoder/actions/workflows/rust.yml/badge.svg)](https://github.com/DoumanAsh/mp3lame-encoder/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/mp3lame-encoder.svg)](https://crates.io/crates/mp3lame-encoder)
[![Documentation](https://docs.rs/mp3lame-encoder/badge.svg)](https://docs.rs/crate/mp3lame-encoder/)

High level wrapper over [mp3lame-sys](https://crates.io/crates/mp3lame-sys)

## Example

```rust
use mp3lame_encoder::{Builder, Id3Tag, DualPcm, FlushNoGap};

let mut mp3_encoder = Builder::new().expect("Create LAME builder");
mp3_encoder.set_num_channels(2).expect("set channels");
mp3_encoder.set_sample_rate(44_100).expect("set sample rate");
mp3_encoder.set_brate(mp3lame_encoder::Bitrate::Kbps192).expect("set brate");
mp3_encoder.set_quality(mp3lame_encoder::Quality::Best).expect("set quality");
mp3_encoder.set_id3_tag(Id3Tag {
    title: b"My title",
    artist: &[],
    album: b"My album",
    year: b"Current year",
    comment: b"Just my comment",
});
let mut mp3_encoder = mp3_encoder.build().expect("To initialize LAME encoder");

//use actual PCM data
let input = DualPcm {
    left: &[0u16, 0],
    right: &[0u16, 0],
};

let mut mp3_out_buffer = Vec::new();
mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(input.left.len()));
let encoded_size = mp3_encoder.encode(input, mp3_out_buffer.spare_capacity_mut()).expect("To encode");
unsafe {
    mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
}

let encoded_size = mp3_encoder.flush::<FlushNoGap>(mp3_out_buffer.spare_capacity_mut()).expect("to flush");
unsafe {
    mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
}
//At this point your mp3_out_buffer should have full MP3 data, ready to be written on file system or whatever

if mp3_encoder.lame_tag_size() > 0 {
   let id3v2_tag_boundary = mp3_encoder.id3v2_tag_size();
   assert_eq!(id3v2_tag_boundary, 158);
   let mut lame_tag = [core::mem::MaybeUninit::uninit(); 1024];
   assert!(lame_tag.len() >= mp3_encoder.lame_tag_size(), "Increase buffer size");
   let lame_tag_size = mp3_encoder.lame_tag_encode(&mut lame_tag).expect("to write lame tag");
   assert_eq!(mp3_encoder.lame_tag_size(), lame_tag_size.get());

   //If you need VBR tag then you need to write mp3 file in following order
   //- id3v2 tag
   //- VBR tag
   //- actual mp3 content
   let chunks_to_write = [
       &mp3_out_buffer[..id3v2_tag_boundary],
       unsafe {
           core::slice::from_raw_parts(lame_tag.as_ptr() as *const u8, lame_tag_size.get())
       },
       &mp3_out_buffer[id3v2_tag_boundary..],
   ];
} else {
   let chunks_to_write = [
       &mp3_out_buffer[..]
   ];
}
```

## License

LAME library is under LGPL License.
Hence this crate is licensed under the same shitty license
