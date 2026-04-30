//! Build script for mali-g68-mp5-driver
//!
//! Generates FFI bindings for DRM and Android native APIs.

fn main() {
    // Generate bindings for DRM interfaces
    // In production, this uses bindgen to wrap:
    // - drm.h (DRM core ioctls)
    // - drm_mode.h (DRM KMS)
    // - panfrost_drm.h (Panfrost-specific ioctls)
    // - android/hardware_buffer.h (Android NDK)
    // - android/native_window.h (Android NDK)
    println!("cargo:rerun-if-changed=build.rs");

    // Link against Android libraries when building for Android
    #[cfg(target_os = "android")]
    {
        println!("cargo:rustc-link-lib=android");
        println!("cargo:rustc-link-lib=log");
        println!("cargo:rustc-link-lib=nativewindow");
    }
}