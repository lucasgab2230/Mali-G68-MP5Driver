# 🔥 Mali-G68 MP5 Vulkan Driver

**Open-source Vulkan driver for ARM Mali-G68 MP5 GPU, written in Rust and optimized for emulators.**

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://rust-lang.org)
[![Vulkan](https://img.shields.io/badge/Vulkan-1.3-red.svg)](https://vulkan.io)

## 🎯 Goal

Bring **Turnip-level** Vulkan driver performance to devices with Mali-G68 MP5 GPUs (Samsung Galaxy A26 5G, A53 5G, etc.). Like Turnip did for Adreno GPUs, this driver aims to make emulators run smoothly on Mali hardware.

## 📱 Target Device

| Specification | Value |
|---|---|
| **Phone** | Samsung Galaxy A26 5G |
| **SoC** | Exynos 1280 |
| **GPU** | ARM Mali-G68 MP5 (Valhall Gen2) |
| **Shader Cores** | 5 |
| **L2 Cache** | 512 KB |
| **Max Frequency** | ~950 MHz |
| **Architecture** | Valhall (2nd Gen) |
| **AFBC** | v1.3 (lossless + wide block) |

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Vulkan 1.3 API                        │
│  Instance · PhysicalDevice · Device · Swapchain          │
├─────────────────────────────────────────────────────────┤
│                  Emulator Optimizations                   │
│  Pipeline Cache · Async Compute · Draw Batching           │
├─────────────────────────────────────────────────────────┤
│                  Shader Compiler                          │
│  SPIR-V → NIR → Valhall ISA (with emulator passes)       │
├─────────────────────────────────────────────────────────┤
│                  Command Stream Frontend (CSF)            │
│  Graphics Queue · Compute Queue · Transfer Queue          │
├─────────────────────────────────────────────────────────┤
│                  Memory Management                        │
│  Buffer Objects · Slab Allocator · Memory Pools · MMU     │
├─────────────────────────────────────────────────────────┤
│                  GPU Hardware Layer                       │
│  Registers · Tiler · Shader Cores · L2 Cache · AFBC      │
└─────────────────────────────────────────────────────────┘
```

## 🎮 Emulator Optimizations

This driver includes **specific optimizations for emulator workloads**:

### Pipeline Cache
- Caches compiled shader programs keyed by SPIR-V hash + state
- Avoids recompilation of identical shaders across draws
- LRU eviction with 512-entry default capacity
- Disk serialization for persistence across restarts

### Async Compute Texture Decode
- Decodes compressed textures (BC1-BC7, ASTC, ETC2) on the **compute queue**
- Overlaps texture upload with rendering on the **graphics queue**
- Supported formats: BC1-BC7, ASTC 4x4/6x6/8x8, ETC2, EAC
- Wavefront-optimized workgroups for Valhall (W8 = 8 threads)

### Shader Compiler Emulator Passes
- **Vertex Transform**: FMA chain optimization for matrix multiplies
- **Texture Decode Compute**: Shared memory optimization, W8 alignment
- **Fragment Texturing**: FP16 conversion, dual-issue scheduling
- **Post-Processing**: Loop unrolling for blur kernels
- **UBO Constant Folding**: Fold static UBO values into immediates

### AFBC Compression
- Automatically applies ARM Frame Buffer Compression (AFBC) to render targets
- v1.3 lossless compression with wide block support
- Reduces memory bandwidth by 50-70% on color attachments
- Supported formats: RGBA8, BGRA8, RGB565, D24S8

## 📦 Project Structure

```
src/
├── lib.rs              # Main entry point, driver config
├── gpu/                # GPU hardware layer
│   ├── info.rs         # GPU identification & capabilities
│   ├── regs.rs         # Hardware register definitions
│   └── tiler.rs        # Valhall tiler (bin-based rendering)
├── csf/                # Command Stream Frontend
│   ├── queue.rs        # CSF command queue (ring buffer)
│   └── firmware.rs     # CSF firmware interface
├── mem/                # Memory management
│   ├── bo.rs           # Buffer objects (DRM allocation)
│   ├── slab.rs         # Slab allocator
│   └── pool.rs         # Memory pools (typed suballocation)
├── mmu/                # GPU MMU
│   └── as.rs           # Address space management
├── compiler/           # Shader compiler
│   ├── nir.rs          # NIR-like intermediate representation
│   ├── valhall.rs      # Valhall ISA code generation
│   ├── optimize.rs     # Standard optimization passes
│   └── emulator_pass.rs # Emulator-specific optimizations
├── cmd/                # Command buffer recording
│   ├── draw.rs         # Draw commands
│   ├── compute.rs      # Compute dispatch
│   ├── transfer.rs     # Copy/blit commands
│   └── builder.rs      # Command buffer builder
├── emulator/           # Emulator optimizations
│   ├── cache.rs        # Pipeline cache
│   └── async_compute.rs # Async texture decode
├── device/             # Device abstraction
│   ├── init.rs         # Device initialization
│   └── queue.rs        # Device queue wrapper
├── vulkan/             # Vulkan 1.3 implementation
│   ├── instance.rs     # VkInstance
│   ├── physical.rs     # VkPhysicalDevice
│   ├── device.rs       # VkDevice
│   ├── memory.rs       # VkDeviceMemory
│   ├── buffer.rs       # VkBuffer
│   ├── image.rs        # VkImage + AFBC
│   ├── pipeline.rs     # VkPipeline
│   ├── shader.rs       # VkShaderModule
│   ├── descriptor.rs   # VkDescriptorSet
│   ├── render_pass.rs  # VkRenderPass
│   ├── sync.rs         # Fences + Semaphores
│   └── swapchain.rs    # Swapchain / WSI
└── util/               # Utilities
    └── hash.rs         # Fast hash maps (FxHash)
```

## 🔨 Building

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- For Android: Android NDK r25+
- For Linux: DRM development headers

### Desktop Build (Linux)

```bash
# Clone the repository
git clone https://github.com/lucasgab2230/Mali-G68-MP5Driver.git
cd Mali-G68-MP5Driver

# Build
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Android Cross-Compilation

```bash
# Install Android target
rustup target add aarch64-linux-android

# Build for Android (using cargo-ndk)
cargo ndk -t arm64-v8a build --release
```

### Feature Flags

| Flag | Default | Description |
|---|---|---|
| `vulkan_1_3` | ✓ | Enable Vulkan 1.3 features |
| `vulkan_1_2` | ✗ | Vulkan 1.2 only |
| `vulkan_1_1` | ✗ | Vulkan 1.1 only |
| `debug_cmds` | ✗ | Dump command stream for debugging |
| `trace` | ✗ | Enable command stream tracing |

## 🧪 Testing

```bash
# Run all unit tests
cargo test

# Run integration tests only
cargo test --test basic

# Run with trace output
RUST_LOG=mali_g68=trace cargo test

# Run benchmarks
cargo bench
```

## 📊 Performance Targets

| Metric | Target | Notes |
|---|---|---|
| Shader compile time | < 5ms (cached: < 0.1ms) | Pipeline cache hit |
| Draw call overhead | < 10μs | Command stream submission |
| Texture decode (BC3 256x256) | < 0.5ms | Async compute on Queue 1 |
| AFBC bandwidth savings | 50-70% | On color attachments |
| Pipeline cache hit rate | > 90% | After warm-up frames |

## 🗺️ Roadmap

- [x] GPU hardware layer (registers, info, tiler)
- [x] CSF command queue and firmware interface
- [x] Memory management (BO, slab allocator, pools)
- [x] GPU MMU and address spaces
- [x] Shader compiler (SPIR-V → NIR → Valhall ISA)
- [x] Shader optimization passes (DCE, constant folding, CSE)
- [x] Emulator-specific shader passes
- [x] Command buffer recording
- [x] Vulkan instance, physical device, device
- [x] Vulkan resources (buffers, images, memory)
- [x] Vulkan pipelines, shaders, descriptors
- [x] Pipeline cache
- [x] Async compute texture decode
- [x] AFBC compression support
- [ ] DRM/KMS backend for real hardware
- [ ] Android BufferQueue integration
- [ ] Full Valhall ISA encoder (verified against hardware)
- [ ] Conformance tests (dEQP-VK)
- [ ] Real hardware testing on Galaxy A26 5G

## 🤝 Contributing

Contributions are welcome! This is a community-driven project to bring better GPU driver support to Mali devices.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing`)
3. Commit your changes (`git commit -m "Add amazing feature"`)
4. Push to the branch (`git push origin feature/amazing`)
5. Open a Pull Request

## 📄 License

Licensed under either of:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

## 🙏 Acknowledgments

- **Mesa / Freedreno / Turnip** - Reference architecture for open-source GPU drivers
- **Panfrost** - ARM Mali reverse-engineering and Valhall documentation
- **ARM** - Mali GPU Architecture Reference Manual
- **Vulkan** - Khronos Vulkan API specification

---

*Built with ❤️ for the emulation community. Let's make Mali GPUs run emulators as well as Turnip does for Adreno!*
