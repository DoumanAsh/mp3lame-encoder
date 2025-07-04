//!High level wrapper over [mp3lame-sys](https://crates.io/crates/mp3lame-sys)
//!
//!## Example
//!
//!```rust
//!use mp3lame_encoder::{Builder, Id3Tag, DualPcm, FlushNoGap};
//!
//!let mut mp3_encoder = Builder::new().expect("Create LAME builder");
//!mp3_encoder.set_num_channels(2).expect("set channels");
//!mp3_encoder.set_sample_rate(44_100).expect("set sample rate");
//!mp3_encoder.set_brate(mp3lame_encoder::Bitrate::Kbps192).expect("set brate");
//!mp3_encoder.set_quality(mp3lame_encoder::Quality::Best).expect("set quality");
//!mp3_encoder.set_id3_tag(Id3Tag {
//!    title: b"My title",
//!    artist: &[],
//!    album: b"My album",
//!    album_art: &[],
//!    year: b"Current year",
//!    comment: b"Just my comment",
//!});
//!let mut mp3_encoder = mp3_encoder.build().expect("To initialize LAME encoder");
//!
//!//use actual PCM data
//!let input = DualPcm {
//!    left: &[0u16, 0],
//!    right: &[0u16, 0],
//!};
//!
//!let mut mp3_out_buffer = Vec::new();
//!mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(input.left.len()));
//!let encoded_size = mp3_encoder.encode(input, mp3_out_buffer.spare_capacity_mut()).expect("To encode");
//!unsafe {
//!    mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
//!}
//!
//!let encoded_size = mp3_encoder.flush::<FlushNoGap>(mp3_out_buffer.spare_capacity_mut()).expect("to flush");
//!unsafe {
//!    mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
//!}
//!//At this point your mp3_out_buffer should have full MP3 data, ready to be written on file system or whatever
//!
//!```

#![no_std]
#![warn(missing_docs)]
#![allow(clippy::style)]
#![allow(clippy::missing_safety_doc)]
#![cfg_attr(rustfmt, rustfmt_skip)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub use mp3lame_sys as ffi;

use alloc::vec::Vec;
use core::mem::{self, MaybeUninit};
use core::ptr::{self, NonNull};
use core::{cmp, fmt};

mod input;
pub use input::*;

///Maximum size of album art
pub const MAX_ALBUM_ART_SIZE: usize = 128 * 1024;

///Calculates maximum required size for specified number of samples.
///
///Note that actual requirement may vary depending on encoder parameters,
///but this size should be generally enough for encoding given number of samples
pub const fn max_required_buffer_size(sample_number: usize) -> usize {
    //add 25% sample number + mp3 frame size 7200
    let mut sample_extra_size = sample_number / 4;
    if (sample_number % 4) > 0 {
        sample_extra_size = sample_extra_size.wrapping_add(1);
    }
    sample_number.wrapping_add(sample_extra_size).wrapping_add(7200)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
///ID3 setter errors
pub enum Id3TagError {
    ///Specified buffer exceed limit of 128kb
    AlbumArtOverflow,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
///Encoder builder errors
pub enum BuildError {
    ///Generic error, indicates invalid input or state
    Generic,
    ///Failed to allocate memory
    NoMem,
    ///Invalid brate
    BadBRate,
    ///Invalid sample frequency
    BadSampleFreq,
    ///Internal error
    InternalError,
    ///Other errors, most likely unexpected.
    Other(libc::c_int),
}

impl BuildError {
    #[inline(always)]
    fn from_c_int(code: libc::c_int) -> Result<(), Self> {
        if code >= 0 {
            return Ok(())
        }

        match code {
            -1 => Err(Self::Generic),
            -10 => Err(Self::NoMem),
            -11 => Err(Self::BadBRate),
            -12 => Err(Self::BadSampleFreq),
            -13 => Err(Self::InternalError),
            _ => Err(Self::Other(code)),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BuildError {
}

impl fmt::Display for BuildError {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Generic => fmt.write_str("error"),
            Self::NoMem => fmt.write_str("alloc failure"),
            Self::BadBRate => fmt.write_str("bad bitrate"),
            Self::BadSampleFreq => fmt.write_str("bad sample frequency"),
            Self::InternalError => fmt.write_str("internal error"),
            Self::Other(code) => fmt.write_fmt(format_args!("error code={code}")),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
///Encoder errors
pub enum EncodeError {
    ///Indicates output buffer is insufficient.
    ///
    ///Consider using [max_required_buffer_size](max_required_buffer_size) to determine required
    ///space to alloc.
    BufferTooSmall,
    ///Failed to allocate memory
    NoMem,
    ///Invalid encoder state
    ///
    ///Should not happen if encoder created through builder
    InvalidState,
    ///Psycho acoustic problems, whatever it means.
    PsychoAcoustic,
    ///Other errors, most likely unexpected.
    Other(libc::c_int),
}

impl EncodeError {
    #[inline(always)]
    fn from_c_int(code: libc::c_int) -> Result<usize, Self> {
        if code >= 0 {
            return Ok(code as usize)
        }

        match code {
            -1 => Err(Self::BufferTooSmall),
            -2 => Err(Self::NoMem),
            -3 => Err(Self::InvalidState),
            -4 => Err(Self::PsychoAcoustic),
            _ => Err(Self::Other(code)),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {
}

impl fmt::Display for EncodeError {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BufferTooSmall => fmt.write_str("output buffer is insufficient for encoder output"),
            Self::NoMem => fmt.write_str("alloc failure"),
            Self::InvalidState => fmt.write_str("attempt to use uninitialized encoder"),
            Self::PsychoAcoustic => fmt.write_str("psycho acoustic problems"),
            Self::Other(code) => fmt.write_fmt(format_args!("error code={code}")),
        }
    }
}


///Enumeration of valid values for `set_brate`
#[derive(Copy, Clone)]
#[repr(u16)]
pub enum Bitrate {
    ///8_000
    Kbps8 = 8,
    ///16_000
    Kbps16 = 16,
    ///24_000
    Kbps24 = 24,
    ///32_000
    Kbps32 = 32,
    ///40_000
    Kbps40 = 40,
    ///48_000
    Kbps48 = 48,
    ///64_000
    Kbps64 = 64,
    ///80_000
    Kbps80 = 80,
    ///96_000
    Kbps96 = 96,
    ///112_000
    Kbps112 = 112,
    ///128_000
    Kbps128 = 128,
    ///160_000
    Kbps160 = 160,
    ///192_000
    Kbps192 = 192,
    ///224_000
    Kbps224 = 224,
    ///256_000
    Kbps256 = 256,
    ///320_000
    Kbps320 = 320,
}

///Alias to `Bitrate` with incorrect spelling
pub use Bitrate as Birtate;

#[derive(Copy, Clone)]
#[repr(u8)]
///Possible VBR types
pub enum VbrMode {
    ///Off.
    Off = ffi::vbr_mode::vbr_off as u8,
    ///MT.
    Mt = ffi::vbr_mode::vbr_mt as u8,
    ///RH.
    Rh = ffi::vbr_mode::vbr_rh as u8,
    ///ABR.
    Abr = ffi::vbr_mode::vbr_abr as u8,
    ///MTRH.
    Mtrh = ffi::vbr_mode::vbr_mtrh as u8,
}

impl Default for VbrMode {
    #[inline(always)]
    fn default() -> Self {
        Self::Mtrh
    }
}

#[derive(Copy, Clone)]
#[repr(u8)]
///Possible modes for encoder
pub enum Mode {
    ///Mono.
    Mono = ffi::MPEG_mode::MONO as u8,
    ///Stereo.
    Stereo = ffi::MPEG_mode::STEREO as u8,
    ///Joint stereo.
    JointStereo = ffi::MPEG_mode::JOINT_STEREO as u8,
    ///Dual channel
    ///
    ///Unsupported so far.
    DaulChannel = ffi::MPEG_mode::DUAL_CHANNEL as u8,
    ///Not set.
    NotSet = ffi::MPEG_mode::NOT_SET as u8,
}

///Possible quality parameter.
///From best(0) to worst(9)
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Quality {
    ///Best possible quality
    Best = 0,
    ///Second best
    SecondBest = 1,
    ///Close to best
    NearBest = 2,
    ///Very nice
    VeryNice = 3,
    ///Nice
    Nice = 4,
    ///Good
    Good = 5,
    ///Decent
    Decent = 6,
    ///Okayish
    Ok = 7,
    ///Almost worst
    SecondWorst = 8,
    ///Worst
    Worst = 9,
}

///ID3 tag as raw bytes.
///
///Use empty slice for `None`
///
///At the current moment, only up to 250 characters will be copied.
pub struct Id3Tag<'a> {
    ///Track's Title
    pub title: &'a [u8],
    ///Artist name
    pub artist: &'a [u8],
    ///Album name
    pub album: &'a [u8],
    ///Album art
    ///
    ///Must be image data.
    ///
    ///Allowed formats: PNG, JPG, GIF
    ///
    ///Maximum size is defined by constant MAX_ALBUM_ART_SIZE
    ///When setting this metadata, make sure allocate at least MAX_ALBUM_ART_SIZE
    pub album_art: &'a [u8],
    ///Year
    pub year: &'a [u8],
    ///Comment
    pub comment: &'a [u8],
}

impl Id3Tag<'_> {
    #[inline(always)]
    ///Returns true if any is set
    pub const fn is_any_set(&self) -> bool {
        !self.title.is_empty() || !self.artist.is_empty() || !self.album.is_empty() || !self.album_art.is_empty() || !self.year.is_empty() || !self.comment.is_empty()
    }
}

///Builder of C LAME encoder.
pub struct Builder {
    inner: NonNull<ffi::lame_global_flags>,
}

impl Builder {
    #[inline]
    ///Creates new encoder with default parameters: J-Stereo, 44.1khz 128kbps CBR mp3 file at quality 5
    ///
    ///Returns `None` if unable to allocate struct.
    pub fn new() -> Option<Self> {
        let ptr = unsafe {
            ffi::lame_init()
        };

        NonNull::new(ptr).map(|inner| Self {
            inner
        })
    }

    #[inline(always)]
    ///Get access to underlying LAME structure, without dropping ownership.
    ///
    ///User must guarantee not to close or dealloc this pointer
    pub unsafe fn as_ptr(&mut self) -> *mut ffi::lame_global_flags {
        self.ptr()
    }

    #[inline(always)]
    fn ptr(&mut self) -> *mut ffi::lame_global_flags {
        self.inner.as_ptr()
    }

    #[inline]
    ///Sets sample rate.
    ///
    ///Defaults to 44_100.
    ///
    ///Returns whether it is supported or not.
    pub fn set_sample_rate(&mut self, rate: u32) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_in_samplerate(self.ptr(), rate.try_into().unwrap_or(libc::c_int::MAX))
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets sample rate using the builder pattern.
    /// 
    ///Defaults to 44_100.
    /// 
    ///Returns an error if it is not supported.
    pub fn with_sample_rate(mut self, rate: u32) -> Result<Self, BuildError> {
        self.set_sample_rate(rate)?;
        Ok(self)
    }

    #[inline]
    ///Sets number of channels.
    ///
    ///Defaults to 2.
    ///
    ///Returns whether it is supported or not.
    pub fn set_num_channels(&mut self, num: u8) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_num_channels(self.ptr(), num as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets sample rate using the builder pattern.
    ///
    ///Defaults to 2.
    ///
    ///Returns an error if it is not supported.
    pub fn with_num_channels(mut self, num: u8) -> Result<Self, BuildError> {
        self.set_num_channels(num)?;
        Ok(self)
    }

    #[inline]
    ///Sets bitrate (as kbps).
    ///
    ///Defaults to compression ratio of 11.
    ///
    ///Returns whether it is supported or not.
    pub fn set_brate(&mut self, brate: Bitrate) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_brate(self.ptr(), brate as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets bitrate (as kbps) using the builder pattern.
    ///
    ///Defaults to compression ratio of 11.
    ///
    ///Returns an error if it is not supported.
    pub fn with_brate(mut self, brate: Bitrate) -> Result<Self, BuildError> {
        self.set_brate(brate)?;
        Ok(self)
    }

    #[inline]
    ///Sets MPEG mode.
    ///
    ///Default is picked by LAME depending on compression ration and input channels.
    ///
    ///Returns whether it is supported or not.
    pub fn set_mode(&mut self, mode: Mode) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_mode(self.ptr(), mode as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets MPEG mode using the builder pattern.
    ///
    ///Default is picked by LAME depending on compression ration and input channels.
    ///
    ///Returns an error if it is not supported.
    pub fn with_mode(mut self, mode: Mode) -> Result<Self, BuildError> {
        self.set_mode(mode)?;
        Ok(self)
    }

    #[inline]
    ///Sets quality.
    ///
    ///Default is good one(5)
    ///
    ///Returns whether it is supported or not.
    pub fn set_quality(&mut self, quality: Quality) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_quality(self.ptr(), quality as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets quality using the builder pattern.
    ///
    ///Default is good one(5)
    ///
    ///Returns an error if it is not supported.
    pub fn with_quality(mut self, quality: Quality) -> Result<Self, BuildError> {
        self.set_quality(quality)?;
        Ok(self)
    }

    #[inline]
    ///Sets VBR quality.
    ///
    ///Returns whether it is supported or not.
    pub fn set_vbr_quality(&mut self, quality: Quality) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_VBR_q(self.ptr(), quality as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets VBR quality using the builder pattern.
    /// 
    ///Returns an error if it is not supported.
    pub fn with_vbr_quality(mut self, quality: Quality) -> Result<Self, BuildError> {
        self.set_vbr_quality(quality)?;
        Ok(self)
    }

    #[inline]
    ///Sets whether to write VBR tag.
    ///
    ///Default is true.
    ///
    ///Returns whether it is supported or not.
    pub fn set_to_write_vbr_tag(&mut self, value: bool) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_bWriteVbrTag(self.ptr(), value as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets whether to write VBR tag using the builder pattern.
    ///
    ///Default is true.
    ///
    ///Returns an error if it is not supported.
    pub fn with_to_write_vbr_tag(mut self, value: bool) -> Result<Self, BuildError> {
        self.set_to_write_vbr_tag(value)?;
        Ok(self)
    }

    #[inline]
    ///Sets VBR mode.
    ///
    ///Default is off (i.e. CBR)
    ///
    ///Returns whether it is supported or not.
    pub fn set_vbr_mode(&mut self, value: VbrMode) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_VBR(self.ptr(), value as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets VBR mode using the bulder pattern.
    /// 
    ///Default is off (i.e. CBR)
    /// 
    ///Returns an error if it is not supported.
    pub fn with_vbr_mode(mut self, value: VbrMode) -> Result<Self, BuildError> {
        self.set_vbr_mode(value)?;
        Ok(self)
    }

    #[inline]
    ///Sets id3tag tag.
    ///
    ///If [FlushGap](FlushGap) is used, then `v1` will not be added.
    ///But `v2` is always added at the beginning.
    ///
    ///Returns whether it is supported or not.
    pub fn set_id3_tag(&mut self, value: Id3Tag<'_>) -> Result<(), Id3TagError> {
        if !value.is_any_set() {
            return Ok(());
        }

        const MAX_BUFFER: usize = 250;
        let mut buffer = [0u8; MAX_BUFFER + 1];

        unsafe {
            ffi::id3tag_init(self.ptr());
            ffi::id3tag_add_v2(self.ptr());

            if !value.album_art.is_empty() {
                let size = value.album_art.len();
                if size > MAX_ALBUM_ART_SIZE {
                    return Err(Id3TagError::AlbumArtOverflow);
                }
                let ptr = value.album_art.as_ptr();
                ffi::id3tag_set_albumart(self.ptr(), ptr as _, size);
            }

            if !value.title.is_empty() {
                let size = cmp::min(MAX_BUFFER, value.title.len());
                ptr::copy_nonoverlapping(value.title.as_ptr(), buffer.as_mut_ptr(), size);
                buffer[size] = 0;
                ffi::id3tag_set_title(self.ptr(), buffer.as_ptr() as _);
            }

            if !value.album.is_empty() {
                let size = cmp::min(MAX_BUFFER, value.album.len());
                ptr::copy_nonoverlapping(value.album.as_ptr(), buffer.as_mut_ptr(), size);
                buffer[size] = 0;
                ffi::id3tag_set_album(self.ptr(), buffer.as_ptr() as _);
            }

            if !value.artist.is_empty() {
                let size = cmp::min(MAX_BUFFER, value.artist.len());
                ptr::copy_nonoverlapping(value.artist.as_ptr(), buffer.as_mut_ptr(), size);
                buffer[size] = 0;
                ffi::id3tag_set_artist(self.ptr(), buffer.as_ptr() as _);
            }

            if !value.year.is_empty() {
                let size = cmp::min(MAX_BUFFER, value.year.len());
                ptr::copy_nonoverlapping(value.year.as_ptr(), buffer.as_mut_ptr(), size);
                buffer[size] = 0;
                ffi::id3tag_set_year(self.ptr(), buffer.as_ptr() as _);
            }

            if !value.comment.is_empty() {
                let size = cmp::min(MAX_BUFFER, value.comment.len());
                ptr::copy_nonoverlapping(value.comment.as_ptr(), buffer.as_mut_ptr(), size);
                buffer[size] = 0;
                ffi::id3tag_set_comment(self.ptr(), buffer.as_ptr() as _);
            }
        }

        Ok(())
    }

    #[inline]
    ///Sets id3tag tag using the builder pattern.
    ///
    ///If [FlushGap](FlushGap) is used, then `v1` will not be added.
    /// 
    ///Returns an error if it is not supported.
    pub fn with_id3_tag(mut self, value: Id3Tag<'_>) -> Result<Self, Id3TagError> {
        self.set_id3_tag(value)?;
        Ok(self)
    }

    #[inline]
    ///Attempts to initialize encoder with specified parameters.
    ///
    ///Returns `None` if parameters are invalid or incompatible.
    pub fn build(mut self) -> Result<Encoder, BuildError> {
        let res = unsafe {
            ffi::lame_init_params(self.ptr())
        };

        match BuildError::from_c_int(res) {
            Ok(()) => {
                let inner = self.inner;
                mem::forget(self);
                Ok(Encoder { inner })
            },
            Err(error) => Err(error),
        }
    }
}

impl Drop for Builder {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            ffi::lame_close(self.ptr())
        };
    }
}

///LAME Encoder.
pub struct Encoder {
    inner: NonNull<ffi::lame_global_flags>,
}

impl Encoder {
    #[inline(always)]
    fn ptr(&self) -> *mut ffi::lame_global_flags {
        self.inner.as_ptr()
    }

    #[inline]
    ///Returns sample rate.
    pub fn sample_rate(&self) -> u32 {
        unsafe {
            ffi::lame_get_in_samplerate(self.ptr()) as u32
        }
    }

    #[inline]
    ///Returns number of channels.
    pub fn num_channels(&self) -> u8 {
        unsafe {
            ffi::lame_get_num_channels(self.ptr()) as u8
        }
    }

    #[inline]
    ///Attempts to encode PCM data, writing whatever available onto `output` buffer
    ///
    ///### Arguments:
    ///
    /// - `input` - Data input. Can be [MonoPcm](MonoPcm), [DualPcm](DualPcm) or [InterleavedPcm](InterleavedPcm)
    /// - `output` - Output buffer to write into.
    ///
    ///### Result:
    ///On success, returns number of bytes written (can be 0).
    ///Otherwise returns error indicating potential issue.
    pub fn encode(&mut self, input: impl EncoderInput, output: &mut [MaybeUninit<u8>]) -> Result<usize, EncodeError> {
        let output_len = output.len();
        let output_buf = output.as_mut_ptr();

        let result = input.encode(self, output_buf as _, output_len);

        EncodeError::from_c_int(result)
    }

    #[inline(always)]
    ///Attempts to encode PCM data, writing whatever available onto `output` buffer
    ///
    ///`output` size is adjusted on success only
    ///
    ///Refer for details to `encode()`
    pub fn encode_to_vec(&mut self, input: impl EncoderInput, output: &mut Vec<u8>) -> Result<usize, EncodeError> {
        let original_len = output.len();
        match self.encode(input, output.spare_capacity_mut()) {
            Ok(written) => {
                unsafe {
                    output.set_len(original_len.saturating_add(written));
                }
                Ok(written)
            },
            Err(error) => Err(error),
        }
    }

    #[inline]
    ///Attempts flush all data, writing whatever available onto `output` buffer
    ///Padding with 0 to complete MP3
    ///
    ///### Type:
    ///
    ///- [FlushNoGap](FlushNoGap) - performs flush, using ancillary data to fill gaps;
    ///- [FlushGap](FlushGap) - performs flush, padding with 0;
    ///
    ///### Arguments:
    ///
    /// - `output` - Output buffer to write into. As it is final action, you need at least 7200 bytes to hold at MP3 data.
    ///
    ///### Result:
    ///On success, returns number of bytes written (can be 0).
    ///Otherwise returns error indicating potential issue.
    pub fn flush<T: EncoderFlush>(&mut self, output: &mut [MaybeUninit<u8>]) -> Result<usize, EncodeError> {
        let output_len = output.len();
        let output_buf = output.as_mut_ptr();

        let result = T::flush(self, output_buf as _, output_len);

        EncodeError::from_c_int(result)
    }

    #[inline(always)]
    ///Attempts flush all data, writing whatever available onto `output` buffer.
    ///
    ///`output` size is adjusted on success only
    ///
    ///Refer for details to `flush()`
    pub fn flush_to_vec<T: EncoderFlush>(&mut self, output: &mut Vec<u8>) -> Result<usize, EncodeError> {
        let original_len = output.len();
        match self.flush::<T>(output.spare_capacity_mut()) {
            Ok(written) => {
                unsafe {
                    output.set_len(original_len.saturating_add(written));
                }
                Ok(written)
            },
            Err(error) => Err(error),
        }
    }
}

impl Drop for Encoder {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            ffi::lame_close(self.ptr())
        };
    }
}

/// According to LAME 3.99.5 HACKING, it is thread-safe.
unsafe impl Send for Encoder {}
/// According to LAME 3.99.5 HACKING, it is thread-safe.
unsafe impl Sync for Encoder {}

///Creates default encoder with 192kbps bitrate and best possible quality.
pub fn encoder() -> Result<Encoder, BuildError> {
    match Builder::new() {
        Some(mut builder) => {
            builder.set_brate(Bitrate::Kbps192)?;
            builder.set_quality(Quality::Best)?;
            builder.build()
        },
        None => Err(BuildError::NoMem)
    }
}
