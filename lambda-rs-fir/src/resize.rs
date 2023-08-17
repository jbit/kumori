use anyhow::Result;
use fast_image_resize::{FilterType, PixelType, ResizeAlg};
use image::codecs::jpeg::JpegEncoder;
use image::{load_from_memory_with_format, ColorType, ImageEncoder, ImageFormat};
use std::io::BufWriter;
use tracing::debug;

pub fn resize_jpeg(input_data: Vec<u8>, dst_width: u32, dst_height: u32) -> Result<Vec<u8>> {
    let input_kb = input_data.len() as f32 / 1024.0;
    debug!("Input JPEG size: {input_kb:.2}KiB");

    let loaded_image = load_from_memory_with_format(&input_data, ImageFormat::Jpeg)?;

    let src_image = fast_image_resize::Image::from_vec_u8(
        loaded_image.width().try_into()?,
        loaded_image.height().try_into()?,
        loaded_image.to_rgb8().into_raw(),
        PixelType::U8x3,
    )?;

    let mut dst_image: fast_image_resize::Image = fast_image_resize::Image::new(
        dst_width.try_into()?,
        dst_height.try_into()?,
        src_image.pixel_type(),
    );

    let mut resizer = fast_image_resize::Resizer::new(ResizeAlg::Convolution(FilterType::Lanczos3));

    debug!(
        "Resizing {}x{} -> {}x{} using:{:?} cpu:{:?}",
        src_image.width(),
        src_image.height(),
        dst_image.width(),
        dst_image.height(),
        resizer.algorithm,
        resizer.cpu_extensions()
    );

    resizer.resize(&src_image.view(), &mut dst_image.view_mut())?;

    let mut result_buf = BufWriter::new(Vec::new());
    JpegEncoder::new(&mut result_buf).write_image(
        dst_image.buffer(),
        dst_image.width().into(),
        dst_image.height().into(),
        ColorType::Rgb8,
    )?;

    let output_data = result_buf.into_inner()?;
    let output_kb = output_data.len() as f32 / 1024.0;
    debug!("Output size: {output_kb:.2}KiB");

    Ok(output_data)
}
