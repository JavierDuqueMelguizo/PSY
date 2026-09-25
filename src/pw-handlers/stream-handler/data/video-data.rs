use pipewire::spa::{param::video::VideoFormat, utils::Rectangle};

pub struct VideoData<const DIMENSIONS: usize>{
    pub bytes: bytes::Bytes,
    pub stride: usize
} 

impl<const DIMENSIONS: usize> VideoData<DIMENSIONS>{
    pub fn new(bytes: bytes::Bytes, stride : usize) -> Self{
        Self { 
            bytes : bytes, 
            stride: stride
        }
    } 

    pub fn size(&self) -> usize{
        self.bytes.len() / DIMENSIONS
    }

    pub fn resolution(&self) -> Rectangle{
        let height = self.bytes.len() / self.stride;
        let width = self.bytes.len() / (height * DIMENSIONS);
        Rectangle{
            width: width as u32,
            height: height as u32
        }
    }


    pub fn pixel(&self, x : usize, y : usize) -> Result<[u8;DIMENSIONS], Box<dyn std::error::Error>>{
        let rect = self.resolution();
        if rect.width <= x as u32 { return Err("EL indice X sobrepasa el ancho".into())}
        if rect.height <= y as u32 { return Err("EL indice Y sobrepasa el alto".into())}
        let index = y * self.stride + x * DIMENSIONS;
        self.bytes[index..index + DIMENSIONS]
            .try_into()
            .map_err(|e| format!("Error leyendo pixel: {e}").into())
    }

}