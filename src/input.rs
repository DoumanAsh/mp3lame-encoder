use super::{Encoder, ffi};

use core::ptr;

///Type of PCM input for encoder
///
///Please note that while you can implement your own trait, it is your responsibility to ensure
///that `encode` function is correct and safe.
pub trait EncoderInput {
    ///Encodes `self` using provided encoder.
    ///
    ///## Arguments
    ///
    ///- `output_buf` - is guaranteed to never to be `null`;
    ///- `output_len` - is guaranteed to be capacity of memory pointed by `output_buf`.
    ///
    ///## Returns
    ///
    ///Zero or positive integer to indicate success and number of bytes written.
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int;
}

///PCM data with only 1 channel
///
///In this case, number of samples is always equals to number of samples in slice.
pub struct MonoPcm<'a, T>(pub &'a [T]);

impl EncoderInput for MonoPcm<'_, u16> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_buffer(encoder.ptr(), self.0.as_ptr() as _, ptr::null(), self.0.len() as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for MonoPcm<'_, i16> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_buffer(encoder.ptr(), self.0.as_ptr(), ptr::null(), self.0.len() as _, output_buf as _, output_len as _)
        }
    }
}

//On most platforms it should be i32
impl EncoderInput for MonoPcm<'_, libc::c_int> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_buffer_int(encoder.ptr(), self.0.as_ptr(), ptr::null(), self.0.len() as _, output_buf as _, output_len as _)
        }
    }
}

#[cfg(all(unix, not(target_arch = "x86")))]
//On most unix it should be i64.
//But unclear about other platforms, so it is only implemented there as otherwise it is i32.
impl EncoderInput for MonoPcm<'_, libc::c_long> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_buffer_long2(encoder.ptr(), self.0.as_ptr(), ptr::null(), self.0.len() as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for MonoPcm<'_, f32> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_buffer_ieee_float(encoder.ptr(), self.0.as_ptr(), ptr::null(), self.0.len() as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for MonoPcm<'_, f64> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_buffer_ieee_double(encoder.ptr(), self.0.as_ptr(), ptr::null(), self.0.len() as _, output_buf as _, output_len as _)
        }
    }
}

///PCM data represented by two channels.
///
///Number of samples must be equal between left and right channels.
///
///If you want to feed encoder single PCM data, then use [MonoPcm](MonoPcm)
///In case length of channels is not equal, it will always feed encoder minimum of both length.
///In debug mode it will panic in this case also to warn you of error.
pub struct DualPcm<'a, T> {
    ///left channel PCM data
    pub left: &'a [T],
    ///right channel PCM data
    pub right: &'a [T],
}

impl EncoderInput for DualPcm<'_, i16> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        debug_assert_eq!(self.left.len(), self.right.len());
        let samples_num = core::cmp::min(self.left.len(), self.right.len());
        unsafe {
            ffi::lame_encode_buffer(encoder.ptr(), self.left.as_ptr(), self.right.as_ptr(), samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for DualPcm<'_, u16> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        debug_assert_eq!(self.left.len(), self.right.len());
        let samples_num = core::cmp::min(self.left.len(), self.right.len());
        unsafe {
            ffi::lame_encode_buffer(encoder.ptr(), self.left.as_ptr() as _, self.right.as_ptr() as _, samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for DualPcm<'_, libc::c_int> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        debug_assert_eq!(self.left.len(), self.right.len());
        let samples_num = core::cmp::min(self.left.len(), self.right.len());
        unsafe {
            ffi::lame_encode_buffer_int(encoder.ptr(), self.left.as_ptr(), self.right.as_ptr(), samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for DualPcm<'_, f32> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        debug_assert_eq!(self.left.len(), self.right.len());
        let samples_num = core::cmp::min(self.left.len(), self.right.len());
        unsafe {
            ffi::lame_encode_buffer_ieee_float(encoder.ptr(), self.left.as_ptr() as _, self.right.as_ptr() as _, samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for DualPcm<'_, f64> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        debug_assert_eq!(self.left.len(), self.right.len());
        let samples_num = core::cmp::min(self.left.len(), self.right.len());
        unsafe {
            ffi::lame_encode_buffer_ieee_double(encoder.ptr(), self.left.as_ptr() as _, self.right.as_ptr() as _, samples_num as _, output_buf as _, output_len as _)
        }
    }
}

///PCM data in interleaved form
///
///Interleaved input assumes you have two channels encoded within continuous buffer as sequence pairs: `[<left>, <right>...]`
///Hence, number of samples is always `data.len() / 2`.
///
///If it is not your case, encoding will panic in debug mode, but otherwise you most likely to get incomplete output.
pub struct InterleavedPcm<'a, T>(pub &'a [T]);

impl EncoderInput for InterleavedPcm<'_, i16> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        let samples_num = self.0.len() / 2;
        debug_assert_eq!(self.0.len() % 2, 0);
        //lame_encode_buffer_interleaved() signature takes mutable pointer, but all other functions const*, wtf?
        unsafe {
            ffi::lame_encode_buffer_interleaved(encoder.ptr(), self.0.as_ptr() as _, samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for InterleavedPcm<'_, u16> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        let samples_num = self.0.len() / 2;
        debug_assert_eq!(self.0.len() % 2, 0);
        //lame_encode_buffer_interleaved() signature takes mutable pointer, but all other functions const*, wtf?
        unsafe {
            ffi::lame_encode_buffer_interleaved(encoder.ptr(), self.0.as_ptr() as _, samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for InterleavedPcm<'_, libc::c_int> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        let samples_num = self.0.len() / 2;
        debug_assert_eq!(self.0.len() % 2, 0);
        unsafe {
            ffi::lame_encode_buffer_interleaved_int(encoder.ptr(), self.0.as_ptr(), samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for InterleavedPcm<'_, f32> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        let samples_num = self.0.len() / 2;
        debug_assert_eq!(self.0.len() % 2, 0);
        unsafe {
            ffi::lame_encode_buffer_interleaved_ieee_float(encoder.ptr(), self.0.as_ptr(), samples_num as _, output_buf as _, output_len as _)
        }
    }
}

impl EncoderInput for InterleavedPcm<'_, f64> {
    #[inline(always)]
    fn encode(self, encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        let samples_num = self.0.len() / 2;
        debug_assert_eq!(self.0.len() % 2, 0);
        unsafe {
            ffi::lame_encode_buffer_interleaved_ieee_double(encoder.ptr(), self.0.as_ptr(), samples_num as _, output_buf as _, output_len as _)
        }
    }
}

///Flush method.
pub trait EncoderFlush {
    ///Performs flush, returning result as signed integer.
    ///
    ///## Arguments
    ///
    ///- `output_buf` - is guaranteed to never to be `null`;
    ///- `output_len` - is guaranteed to be capacity of memory pointed by `output_buf`.
    ///
    ///## Returns
    ///
    ///Zero or positive integer to indicate success and number of bytes written.
    fn flush(encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int;
}

///Performs flush, padding gaps with 0
pub struct FlushGap;

impl EncoderFlush for FlushGap {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[inline(always)]
    fn flush(encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_flush(encoder.ptr(), output_buf, output_len as _)
        }
    }
}

///Performs flush, padding it with ancillary data
pub struct FlushNoGap;

impl EncoderFlush for FlushNoGap {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[inline(always)]
    fn flush(encoder: &mut Encoder, output_buf: *mut u8, output_len: usize) -> libc::c_int {
        unsafe {
            ffi::lame_encode_flush_nogap(encoder.ptr(), output_buf, output_len as _)
        }
    }
}
