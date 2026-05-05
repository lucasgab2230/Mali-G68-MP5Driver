#!/bin/bash

# Build script for Mali-G68 MP5 User-Space Driver
# This script builds the user-space library for emulator integration

set -e

echo "🚀 Building Mali-G68 MP5 User-Space Driver..."

# Check if Rust is available
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust compiler not found"
    echo "Please install Rust: https://rustup.rs/"
    exit 1
fi

# Set up Android target if available
if command -v rustup &> /dev/null; then
    TARGET_TRIPLE="aarch64-linux-android"
    if ! rustup target list --installed | grep -q "$TARGET_TRIPLE"; then
        echo "📦 Installing Rust target: $TARGET_TRIPLE"
        rustup target add "$TARGET_TRIPLE"
    fi
fi

# Create output directory
mkdir -p target/release

echo "🔨 Building user-space driver library..."

# Build the user-space library
cargo build --release --manifest-path Cargo_userspace.toml

# Build the integration example
echo "📦 Building Eden integration example..."
cargo build --release --manifest-path Cargo_userspace.toml --example eden_integration

# Create lib directory for emulator integration
mkdir -p lib

# Copy shared library
if [ -f "target/release/libmali_g68_userspace.so" ]; then
    cp target/release/libmali_g68_userspace.so lib/
    echo "✅ User-space library built: lib/libmali_g68_userspace.so"
else
    echo "❌ User-space library build failed"
    exit 1
fi

# Copy integration example
if [ -f "target/release/examples/eden_integration" ]; then
    cp target/release/examples/eden_integration lib/
    echo "✅ Eden integration built: lib/eden_integration"
else
    echo "❌ Eden integration build failed"
    exit 1
fi

# Create header file for C integration
cat > include/mali_g68_userspace.h << 'EOF'
#ifndef MALI_G68_USERSPACE_H
#define MALI_G68_USERSPACE_H

#ifdef __cplusplus
extern "C" {
#endif

// User-space Mali-G68 driver context
typedef struct MaliG68Context MaliG68Context;

// Configuration structure
typedef struct {
    int enable_optimizations;
    int target_fps;
    int memory_pool_size_mb;
    int enable_debug;
    char* drm_device_path;
} MaliG68Config;

// Performance metrics
typedef struct {
    float current_fps;
    float frame_time_ms;
    float gpu_utilization;
    float memory_used_mb;
    int draw_calls_per_frame;
    float cache_hit_rate;
} MaliG68Metrics;

// Main API functions
MaliG68Context* mali_g68_init(const MaliG68Config* config);
int mali_g68_begin_frame(MaliG68Context* ctx);
int mali_g68_end_frame(MaliG68Context* ctx);
int mali_g68_render_2d(MaliG68Context* ctx, 
                        unsigned long texture_addr,
                        unsigned long vertex_buffer_addr,
                        unsigned int width,
                        unsigned int height);
MaliG68Metrics mali_g68_get_metrics(MaliG68Context* ctx);
int mali_g68_cleanup(MaliG68Context* ctx);

#ifdef __cplusplus
}
#endif

#endif // MALI_G68_USERSPACE_H
EOF

echo "✅ C header created: include/mali_g68_userspace.h"

# Create C wrapper for easier integration
cat > src/c_wrapper.c << 'EOF'
#include "include/mali_g68_userspace.h"
#include <stdlib.h>
#include <string.h>

// Simple C wrapper for the Rust user-space driver
extern MaliG68Context* mali_g68_init_from_env() {
    MaliG68Config config = {0};
    
    // Read from environment variables
    if (getenv("MALI_OPT_LEVEL")) {
        config.enable_optimizations = atoi(getenv("MALI_OPT_LEVEL"));
    } else {
        config.enable_optimizations = 1;
    }
    
    if (getenv("MALI_TARGET_FPS")) {
        config.target_fps = atoi(getenv("MALI_TARGET_FPS"));
    } else {
        config.target_fps = 60;
    }
    
    if (getenv("MALI_MEMORY_POOL")) {
        config.memory_pool_size_mb = atoi(getenv("MALI_MEMORY_POOL"));
    } else {
        config.memory_pool_size_mb = 512;
    }
    
    if (getenv("MALI_DEBUG")) {
        config.enable_debug = atoi(getenv("MALI_DEBUG"));
    } else {
        config.enable_debug = 0;
    }
    
    if (getenv("MALI_DRM_DEVICE")) {
        config.drm_device_path = strdup(getenv("MALI_DRM_DEVICE"));
    } else {
        config.drm_device_path = NULL;
    }
    
    return mali_g68_init(&config);
}

// Convenience function for common 2D rendering
extern int mali_g68_render_quad(MaliG68Context* ctx,
                                unsigned long texture_addr,
                                unsigned long vertex_buffer_addr,
                                unsigned int width,
                                unsigned int height) {
    return mali_g68_render_2d(ctx, texture_addr, vertex_buffer_addr, width, height);
}
EOF

echo "✅ C wrapper created: src/c_wrapper.c"

# Create Makefile for easy integration
cat > Makefile << 'EOF'
# Makefile for Mali-G68 MP5 User-Space Driver integration

CC = gcc
CFLAGS = -O3 -fPIC -Wall -Wextra
LDFLAGS = -shared -L./lib -lmali_g68_userspace
INCLUDE = -I./include

# Default target
all: directories libmali_g68_userspace.so eden_integration

# Create necessary directories
directories:
	@mkdir -p lib include

# Build the user-space library
libmali_g68_userspace.so:
	@echo "🔨 Building Mali-G68 user-space library..."
	cargo build --release --manifest-path Cargo_userspace.toml
	@cp target/release/libmali_g68_userspace.so lib/

# Build Eden integration example
eden_integration: libmali_g68_userspace.so
	@echo "📦 Building Eden integration..."
	cargo build --release --manifest-path Cargo_userspace.toml --example eden_integration
	@cp target/release/examples/eden_integration lib/

# Build C wrapper (if needed)
c_wrapper:
	@echo "🔨 Building C wrapper..."
	$(CC) $(CFLAGS) $(INCLUDE) -c src/c_wrapper.c -o c_wrapper.o
	$(CC) $(LDFLAGS) c_wrapper.o -o lib/libmali_g68_c_wrapper.so

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	rm -rf target/
	rm -rf lib/

# Install system-wide (requires root)
install: all
	@echo "📱 Installing Mali-G68 user-space driver..."
	sudo cp lib/libmali_g68_userspace.so /usr/local/lib/
	sudo cp lib/eden_integration /usr/local/bin/
	sudo cp include/mali_g68_userspace.h /usr/local/include/
	sudo ldconfig
	@echo "✅ Installation completed"

# Test build
test: all
	@echo "🧪 Testing build..."
	./lib/eden_integration

# Help target
help:
	@echo "Mali-G68 MP5 User-Space Driver Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build library and examples"
	@echo "  eden_integration  - Build Eden integration example"
	@echo "  c_wrapper        - Build C wrapper library"
	@echo "  clean            - Clean build artifacts"
	@echo "  install          - Install system-wide (requires root)"
	@echo "  test             - Test the build"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Environment Variables:"
	@echo "  MALI_OPT_LEVEL      - Optimization level (0-3, default: 3)"
	@echo "  MALI_TARGET_FPS     - Target FPS (default: 60)"
	@echo "  MALI_MEMORY_POOL    - Memory pool size in MB (default: 512)"
	@echo "  MALI_DEBUG          - Enable debug logging (0/1, default: 0)"
	@echo "  MALI_DRM_DEVICE     - DRM device path (default: auto-detect)"

.PHONY: all directories libmali_g68_userspace.so eden_integration c_wrapper clean install test help
EOF

echo "✅ Makefile created"

# Create package script for emulators
cat > package_for_emulator.sh << 'EOF'
#!/bin/bash

# Package Mali-G68 user-space driver for emulator distribution

set -e

EMULATOR_NAME="$1"
if [ -z "$EMULATOR_NAME" ]; then
    echo "Usage: $0 <emulator_name>"
    echo "Example: $0 eden"
    exit 1
fi

echo "📦 Packaging Mali-G68 driver for $EMULATOR_NAME..."

# Create package directory
PACKAGE_DIR="mali-g68-${EMULATOR_NAME}-package"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

# Copy necessary files
cp lib/libmali_g68_userspace.so "$PACKAGE_DIR/"
cp lib/eden_integration "$PACKAGE_DIR/"
cp include/mali_g68_userspace.h "$PACKAGE_DIR/"
cp README_USERSPACE.md "$PACKAGE_DIR/"
cp Cargo_userspace.toml "$PACKAGE_DIR/"

# Create integration script
cat > "$PACKAGE_DIR/integrate.sh" << 'PACKAGE_EOF'
#!/bin/bash

# Mali-G68 MP5 User-Space Driver Integration Script for $EMULATOR_NAME

set -e

echo "🚀 Integrating Mali-G68 driver with $EMULATOR_NAME..."

# Check if we're in the right directory
if [ ! -f "libmali_g68_userspace.so" ]; then
    echo "❌ Error: Mali-G68 library not found"
    echo "Please run this script from the package directory"
    exit 1
fi

# Create lib directory in emulator
EMULATOR_LIB_DIR="../$EMULATOR_NAME/lib"
mkdir -p "$EMULATOR_LIB_DIR"

# Copy library
cp libmali_g68_userspace.so "$EMULATOR_LIB_DIR/"

# Copy integration binary if available
if [ -f "eden_integration" ]; then
    cp eden_integration "$EMULATOR_LIB_DIR/"
fi

# Copy header
mkdir -p "../$EMULATOR_NAME/include"
cp mali_g68_userspace.h "../$EMULATOR_NAME/include/"

echo "✅ Mali-G68 driver integrated with $EMULATOR_NAME"
echo ""
echo "📝 Integration Instructions:"
echo "1. Add libmali_g68_userspace.so to your emulator's linking"
echo "2. Include mali_g68_userspace.h in your source code"
echo "3. Call mali_g68_init_from_env() to initialize the driver"
echo "4. Use the rendering API for GPU operations"
echo ""
echo "🔧 Environment Variables for Runtime:"
echo "  export MALI_OPT_LEVEL=3      # Maximum optimization"
echo "  export MALI_TARGET_FPS=60     # Target 60 FPS"
echo "  export MALI_MEMORY_POOL=512    # 512MB memory pool"
echo "  export MALI_DEBUG=1           # Enable debug logging"
echo "  export MALI_DRM_DEVICE=/dev/dri/card1  # Specific DRM device"
PACKAGE_EOF

chmod +x "$PACKAGE_DIR/integrate.sh"

# Create README for package
cat > "$PACKAGE_DIR/README_${EMULATOR_NAME}.md" << 'PACKAGE_README_EOF'
# Mali-G68 MP5 User-Space Driver for $EMULATOR_NAME

## Installation

1. Copy all files to your emulator's source directory
2. Run the integration script:
   \`\`\`bash
   ./integrate.sh
   \`\`\`

## Usage

See README_USERSPACE.md for detailed usage instructions.

## Quick Start

\`\`\`c
#include "mali_g68_userspace.h"

int main() {
    // Initialize driver
    MaliG68Context* ctx = mali_g68_init_from_env();
    if (!ctx) {
        printf("Failed to initialize Mali-G68 driver\\n");
        return 1;
    }
    
    // Main emulation loop
    while (running) {
        mali_g68_begin_frame(ctx);
        
        // Your rendering code here
        render_frame(ctx);
        
        mali_g68_end_frame(ctx);
        
        // Get performance metrics
        MaliG68Metrics metrics = mali_g68_get_metrics(ctx);
        printf("FPS: %.1f\\n", metrics.current_fps);
    }
    
    // Cleanup
    mali_g68_cleanup(ctx);
    return 0;
}
\`\`\`
PACKAGE_README_EOF

# Create tarball
tar -czf "${PACKAGE_DIR}.tar.gz" "$PACKAGE_DIR"

echo "✅ Package created: ${PACKAGE_DIR}.tar.gz"
echo ""
echo "📦 Package contents:"
ls -la "$PACKAGE_DIR/"
EOF

chmod +x package_for_emulator.sh

echo ""
echo "✅ User-space driver build completed successfully!"
echo ""
echo "📁 Output files:"
echo "  - lib/libmali_g68_userspace.so (Main library)"
echo "  - lib/eden_integration (Eden integration example)"
echo "  - include/mali_g68_userspace.h (C header)"
echo "  - src/c_wrapper.c (C wrapper)"
echo "  - Makefile (Build system)"
echo "  - build_userspace.sh (Build script)"
echo "  - package_for_emulator.sh (Packaging script)"
echo ""
echo "🎯 Next steps:"
echo "  1. Test: ./lib/eden_integration"
echo "  2. Package: ./package_for_emulator.sh eden"
echo "  3. Integrate: Copy package to emulator and run integrate.sh"
echo ""
echo "📱 For emulator integration:"
echo "  - Link against libmali_g68_userspace.so"
echo "  - Include mali_g68_userspace.h"
echo "  - Call mali_g68_init_from_env() for initialization"
echo "  - Use rendering API for GPU operations"
