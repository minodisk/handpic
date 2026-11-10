use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use gpui::{AppContext, Context, RenderImage};
use image::{Frame, RgbaImage};
use riffle_backend::index::{self, Index};

use crate::{Riffle, guarded, orient};

/// Thumbnail reads out at once, as the Tauri strip's.
pub const MAX_IN_FLIGHT: usize = 4;

/// What a read of one cell's thumbnail came back with.
pub enum Read {
    Thumbnail(Arc<RenderImage>),
    NotScanned,
    /// Why the scan failed on the file, or why its thumbnail did not decode.
    Failed(String),
}

/// The decoded thumbnails of the strip's range, and the failure text of the
/// cells that failed.
#[derive(Default)]
pub struct Thumbs {
    generation: u64,
    images: HashMap<usize, Arc<RenderImage>>,
    failures: HashMap<usize, String>,
}

impl Thumbs {
    /// Forget everything for a new listing; reads still out for the old one
    /// carry an older generation and are dropped. The images to retire.
    pub fn reset(&mut self) -> Vec<Arc<RenderImage>> {
        self.generation += 1;
        self.failures.clear();
        self.images.drain().map(|(_, image)| image).collect()
    }

    /// A re-listing: keep the images of the cells in `moves` (old index to
    /// new), forget the rest, and drop the reads still out. The images to
    /// retire.
    pub fn carry(&mut self, moves: &HashMap<usize, usize>) -> Vec<Arc<RenderImage>> {
        self.generation += 1;
        self.failures.clear();
        let mut retired = Vec::new();
        for (index, image) in std::mem::take(&mut self.images) {
            match moves.get(&index) {
                Some(&to) => {
                    self.images.insert(to, image);
                }
                None => retired.push(image),
            }
        }
        retired
    }

    /// The cells holding an image.
    pub fn loaded(&self) -> impl Iterator<Item = usize> + '_ {
        self.images.keys().copied()
    }

    pub fn is_current(&self, generation: u64) -> bool {
        generation == self.generation
    }

    pub fn image(&self, index: usize) -> Option<&Arc<RenderImage>> {
        self.images.get(&index)
    }

    pub fn failure(&self, index: usize) -> Option<&str> {
        self.failures.get(&index).map(String::as_str)
    }

    /// The image the cell held before, to retire.
    pub fn insert(&mut self, index: usize, image: Arc<RenderImage>) -> Option<Arc<RenderImage>> {
        self.images.insert(index, image)
    }

    pub fn fail(&mut self, index: usize, reason: String) {
        self.failures.insert(index, reason);
    }

    /// Drop the images of cells that left the range. The images to retire.
    pub fn release(&mut self, indices: &[usize]) -> Vec<Arc<RenderImage>> {
        indices
            .iter()
            .filter_map(|index| self.images.remove(index))
            .collect()
    }

    /// Read and decode one cell's thumbnail on the background executor, then
    /// hand it to the view.
    pub fn request(
        &self,
        reader: Arc<Mutex<Index>>,
        index: usize,
        path: String,
        cx: &mut Context<Riffle>,
    ) {
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_spawn(async move { read(&reader, &path) })
                .await;
            this.update(cx, |this, cx| this.thumbnail(generation, index, read, cx))
                .ok();
        })
        .detach();
    }
}

/// The cached thumbnail of `path` through the index's read-only connection,
/// decoded and turned upright. A row without a thumbnail is a file the scan
/// processed and failed on; no row at all is one it has not reached.
fn read(reader: &Mutex<Index>, path: &str) -> Read {
    let thumbnail = index::lock(reader).thumbnail(path);
    match thumbnail {
        Ok((orientation, jpeg)) => match guarded(|| decode(&jpeg, orientation)) {
            Ok(image) => Read::Thumbnail(image),
            Err(e) => Read::Failed(e),
        },
        Err(e) if e.contains("no cached thumbnail") => {
            let entry = index::lock(reader).entry(path);
            Read::Failed(entry.ok().flatten().and_then(|e| e.error).unwrap_or(e))
        }
        Err(_) => Read::NotScanned,
    }
}

fn decode(jpeg: &[u8], orientation: u16) -> Result<Arc<RenderImage>, String> {
    let mut d = mozjpeg::Decompress::new_mem(jpeg)
        .map_err(|e| e.to_string())?
        .to_colorspace(mozjpeg::ColorSpace::JCS_EXT_BGRA)
        .map_err(|e| e.to_string())?;
    let (w, h) = (d.width() as u32, d.height() as u32);
    let bgra = d.read_scanlines::<u8>().map_err(|e| e.to_string())?;
    d.finish().map_err(|e| e.to_string())?;
    let (bgra, w, h) = orient::rotate_bgra(bgra, w, h, orientation);
    let buffer = RgbaImage::from_raw(w, h, bgra).ok_or("the thumbnail decode came up short")?;
    Ok(Arc::new(RenderImage::new(vec![Frame::new(buffer)])))
}

#[cfg(test)]
mod tests {
    use super::Thumbs;
    use gpui::RenderImage;
    use image::{Frame, RgbaImage};
    use std::sync::Arc;

    fn image() -> Arc<RenderImage> {
        Arc::new(RenderImage::new(vec![Frame::new(RgbaImage::new(1, 1))]))
    }

    #[test]
    fn released_and_reset_images_come_back_to_retire() {
        let mut t = Thumbs::default();
        t.insert(1, image());
        t.insert(2, image());
        t.fail(3, "bad".into());
        assert_eq!(t.release(&[1, 5]).len(), 1);
        assert!(t.image(1).is_none() && t.image(2).is_some());
        assert!(t.is_current(0));
        assert_eq!(t.reset().len(), 1);
        assert!(!t.is_current(0) && t.failure(3).is_none());
    }

    #[test]
    fn carried_images_move_and_the_rest_come_back_to_retire() {
        let mut t = Thumbs::default();
        t.insert(0, image());
        t.insert(1, image());
        t.fail(2, "bad".into());
        let retired = t.carry(&[(1, 0)].into());
        assert_eq!(retired.len(), 1);
        assert!(t.image(0).is_some() && t.image(1).is_none());
        assert_eq!(t.loaded().collect::<Vec<_>>(), [0]);
        assert!(!t.is_current(0) && t.failure(2).is_none());
    }
}
