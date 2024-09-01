use std::ffi::CString;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use thiserror::Error;
use windows::core::PCSTR;

#[derive(Asset, Debug, Clone, TypePath)]
pub struct Shader {
    source: CString,
}

impl Shader {
    pub fn pcstr(&self) -> PCSTR {
        PCSTR::from_raw(self.source.as_ptr() as *const u8)
    }
}

pub struct ShaderLoader;

#[derive(Error, Debug)]
pub enum ShaderError {
    #[error("failed to load file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to convert bytes to string")]
    Utf8(#[from] std::ffi::NulError),
}

impl AssetLoader for ShaderLoader {
    type Asset = Shader;
    type Settings = ();
    type Error = ShaderError;
    async fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Shader, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(Shader {
            source: CString::new(bytes)?,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["hlsl"]
    }
}
