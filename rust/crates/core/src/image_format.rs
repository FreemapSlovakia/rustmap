use napi_derive::napi;

#[derive(Debug, Clone, Copy)]
#[napi(string_enum)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Pdf,
    Svg,
}
