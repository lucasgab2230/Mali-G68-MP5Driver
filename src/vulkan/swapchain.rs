//! Vulkan Swapchain / WSI (Window System Integration)
//!
//! Supports Android (BufferQueue/GRALLOC) and DRM/GBM (Linux)
//! for presenting rendered frames to the display.

/// Swapchain creation parameters
#[derive(Debug, Clone)]
pub struct SwapchainCreateInfo {
    /// Surface width
    pub width: u32,
    /// Surface height
    pub height: u32,
    /// Number of images (double or triple buffering)
    pub image_count: u32,
    /// Image format
    pub format: super::image::ImageFormat,
    /// Presentation mode
    pub present_mode: PresentMode,
    /// Clipped rendering
    pub clipped: bool,
}

/// Presentation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
    /// VSync (waits for next vertical blank)
    Fifo,
    /// VSync with late-swap (reduces latency)
    FifoRelaxed,
    /// No VSync (immediate, may tear)
    Immediate,
    /// VSync with mailbox (lowest latency without tearing)
    Mailbox,
}

impl PresentMode {
    /// Get the optimal present mode for emulators on Mali-G68
    pub fn optimal_for_emulator() -> Self {
        // Mailbox provides the lowest latency for emulator rendering
        // FIFO is the fallback (always supported)
        PresentMode::Mailbox
    }
}

/// Vulkan swapchain
pub struct VkSwapchain {
    /// Swapchain images
    images: Vec<SwapchainImage>,
    /// Current image index
    current_index: u32,
    /// Presentation mode
    present_mode: PresentMode,
}

/// Swapchain image
pub struct SwapchainImage {
    /// Image handle
    pub image: super::image::VkImage,
    /// Buffer queue native buffer handle (Android)
    pub native_buffer: Option<u64>,
}

impl VkSwapchain {
    /// Create a new swapchain
    pub fn new(create_info: &SwapchainCreateInfo) -> Result<Self, SwapchainError> {
        Ok(Self {
            images: Vec::new(),
            current_index: 0,
            present_mode: create_info.present_mode,
        })
    }

    /// Acquire the next image
    pub fn acquire_next_image(&mut self, _timeout_ns: u64) -> Result<u32, SwapchainError> {
        let idx = self.current_index;
        self.current_index = (self.current_index + 1) % (self.images.len().max(1) as u32);
        Ok(idx)
    }

    /// Present the current image
    pub fn present(&self, _image_index: u32) -> Result<(), SwapchainError> {
        Ok(())
    }

    /// Get the number of swapchain images
    pub fn image_count(&self) -> u32 {
        self.images.len() as u32
    }
}

/// Swapchain errors
#[derive(Debug, thiserror::Error)]
pub enum SwapchainError {
    /// Surface lost
    #[error("Surface lost")]
    SurfaceLost,
    /// Out of date (needs recreation)
    #[error("Swapchain out of date")]
    OutOfDate,
    /// Suboptimal (still usable but not ideal)
    #[error("Swapchain suboptimal")]
    Suboptimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_present_mode_optimal() {
        assert_eq!(PresentMode::optimal_for_emulator(), PresentMode::Mailbox);
    }
}