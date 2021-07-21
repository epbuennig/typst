//! Image handling.

use std::collections::{hash_map::Entry, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::io::Cursor;
use std::rc::Rc;

use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};

use crate::loading::{FileId, Loader};

/// A loaded image.
pub struct Image {
    /// The original format the image was encoded in.
    pub format: ImageFormat,
    /// The decoded image.
    pub buf: DynamicImage,
}

impl Image {
    /// Parse an image from raw data in a supported format (PNG or JPEG).
    ///
    /// The image format is determined automatically.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let cursor = Cursor::new(data);
        let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
        let format = reader.format()?;
        let buf = reader.decode().ok()?;
        Some(Self { format, buf })
    }

    /// The width of the image.
    pub fn width(&self) -> u32 {
        self.buf.width()
    }

    /// The height of the image.
    pub fn height(&self) -> u32 {
        self.buf.height()
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Image")
            .field("format", &self.format)
            .field("color", &self.buf.color())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}

/// Caches decoded images.
pub struct ImageCache {
    loader: Rc<dyn Loader>,
    images: HashMap<ImageId, Image>,
    on_load: Option<Box<dyn Fn(ImageId, &Image)>>,
}

impl ImageCache {
    /// Create a new, empty image cache.
    pub fn new(loader: Rc<dyn Loader>) -> Self {
        Self {
            loader,
            images: HashMap::new(),
            on_load: None,
        }
    }

    /// Load and decode an image file from a path.
    pub fn load(&mut self, file: FileId) -> Option<ImageId> {
        let id = ImageId(file.into_raw());
        if let Entry::Vacant(entry) = self.images.entry(id) {
            let buffer = self.loader.load_file(file).ok()?;
            let image = Image::parse(&buffer)?;
            if let Some(callback) = &self.on_load {
                callback(id, &image);
            }
            entry.insert(image);
        }
        Some(id)
    }

    /// Get a reference to a loaded image.
    ///
    /// This panics if no image with this id was loaded. This function should
    /// only be called with ids returned by [`load()`](Self::load).
    #[track_caller]
    pub fn get(&self, id: ImageId) -> &Image {
        &self.images[&id]
    }

    /// Register a callback which is invoked each time an image is loaded.
    pub fn on_load<F>(&mut self, f: F)
    where
        F: Fn(ImageId, &Image) + 'static,
    {
        self.on_load = Some(Box::new(f));
    }
}

/// A unique identifier for a loaded image.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ImageId(u64);

impl ImageId {
    /// Create an image id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub fn from_raw(v: u64) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub fn into_raw(self) -> u64 {
        self.0
    }
}