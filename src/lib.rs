//!High level wrapper over [mp3lame-sys](https://crates.io/crates/mp3lame-sys)

#![warn(missing_docs)]

#![no_std]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]
#![cfg_attr(rustfmt, rustfmt_skip)]

pub use mp3lame_sys as ffi;

use core::mem::{self, MaybeUninit};
use core::ptr::NonNull;
use core::fmt;

mod input;
pub use input::*;

///Calculates maximum required size for specified number of samples.
///
///Note that actual requirement may vary depending on encoder parameters,
///but it should fit any buffer this size.
pub const fn max_required_buffer_size(sample_number: usize) -> usize {
    //add 25% sample number + mp3 frame size 7200
    let mut sample_extra_size = sample_number / 4;
    if (sample_number % 4) > 0 {
        sample_extra_size = sample_extra_size.wrapping_add(1);
    }
    sample_number.wrapping_add(sample_extra_size).wrapping_add(7200)
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

#[cfg(features = "std")]
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
    ///Indicates output buffer is insufficient
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

#[cfg(features = "std")]
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
pub enum Birtate {
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
    fn ptr(&mut self) -> *mut ffi::lame_global_flags {
        self.inner.as_ptr()
    }

    #[inline]
    ///Sets sample rate.
    ///
    ///Defaults to 44_100
    ///
    ///Returns whether it is supported or not.
    pub fn set_sample_rate(&mut self, rate: u32) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_in_samplerate(self.ptr(), rate.try_into().unwrap_or(libc::c_int::max_value()))
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets sample rate.
    ///
    ///Defaults to 2
    ///
    ///Returns whether it is supported or not.
    pub fn set_num_channels(&mut self, num: u8) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_num_channels(self.ptr(), num as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets bitrate (as kbps).
    ///
    ///Defaults to compression ratio of 11.
    ///
    ///Returns whether it is supported or not.
    pub fn set_brate(&mut self, brate: Birtate) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_brate(self.ptr(), brate as _)
        };

        BuildError::from_c_int(res)
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
    ///Sets whether to write VBR tag.
    ///
    ///Default is true
    ///
    ///Returns whether it is supported or not.
    pub fn set_to_write_vbr_tag(&mut self, value: bool) -> Result<(), BuildError> {
        let res = unsafe {
            ffi::lame_set_bWriteVbrTag(self.ptr(), value as _)
        };

        BuildError::from_c_int(res)
    }

    #[inline]
    ///Sets VBR mode
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

///Builder of C LAME encoder.
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
    /// - `input` - Data input. Can be [DualPcm](DualPcm) or [InterleavedPcm](InterleavedPcm)
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
}
