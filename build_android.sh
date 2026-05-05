#!/bin/bash

# Build script for Mali-G68 MP5 Android Shared Library
# This script builds the optimized driver as an Android .so file

set -e

echo "🚀 Building Mali-G68 MP5 Driver for Android..."

# Check if Android NDK is available
if [ -z "$ANDROID_NDK_ROOT" ]; then
    echo "❌ ANDROID_NDK_ROOT environment variable not set"
    echo "Please set ANDROID_NDK_ROOT to your Android NDK path"
    exit 1
fi

# Set target triple for ARM64 Android
TARGET_TRIPLE="aarch64-linux-android"
API_LEVEL=29

echo "📱 Target: $TARGET_TRIPLE (API Level $API_LEVEL)"

# Create output directory
mkdir -p target/aarch64-linux-android/release

# Set up Rust target for Android
if ! rustup target list --installed | grep -q "$TARGET_TRIPLE"; then
    echo "📦 Installing Rust target: $TARGET_TRIPLE"
    rustup target add "$TARGET_TRIPLE"
fi

# Set environment variables for Android build
export CC="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android$API_LEVEL-clang"
export CXX="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android$API_LEVEL-clang++"
export AR="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android$API_LEVEL-clang"

# Build the shared library with optimizations
echo "🔨 Building optimized shared library..."
cargo build --release --target "$TARGET_TRIPLE" --features "vulkan_1_3"

# Copy and rename the shared library
echo "📋 Creating Android shared library..."
cp "target/$TARGET_TRIPLE/release/libmali_g68.so" "libmali_g68.so"

# Create Android.mk for integration
cat > Android.mk << 'EOF'
# Mali-G68 MP5 Vulkan Driver
LOCAL_PATH := $(call my-dir)

include $(CLEAR_VARS)

LOCAL_MODULE := mali_g68
LOCAL_SRC_FILES := libmali_g68.so
LOCAL_MODULE_CLASS := SHARED_LIBRARIES
LOCAL_MODULE_SUFFIX := .so

include $(BUILD_PREBUILT_SHARED_LIBRARY)
EOF

# Create Application.mk for Android build system
cat > Application.mk << 'EOF'
# Mali-G68 MP5 Driver Application Configuration
APP_PLATFORM := android-29
APP_ABI := arm64-v8a
APP_STL := c++_shared
APP_CPPFLAGS := -std=c++17 -O3
EOF

echo "✅ Build completed successfully!"
echo ""
echo "📁 Output files:"
echo "  - libmali_g68.so (Main shared library)"
echo "  - Android.mk (Android build integration)"
echo "  - Application.mk (Android build configuration)"
echo ""
echo "🎯 Performance optimizations included:"
echo "  - Draw call batching"
echo "  - Enhanced pipeline caching"
echo "  - Snapdragon-style optimizations"
echo "  - Doorbell batching"
echo "  - Memory pool optimization"
echo ""
echo "📱 To install on Android:"
echo "  1. Push libmali_g68.so to /system/lib64/"
echo "  2. Set proper permissions (chmod 644)"
echo "  3. Restart graphics services"
