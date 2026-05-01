//! Integration tests for the Mali-G68 MP5 driver

#[cfg(test)]
mod tests {
    use mali_g68::gpu::info::GpuInfo;
    use mali_g68::csf::firmware::CsfFirmware;
    use mali_g68::compiler::nir::{NirShader, ShaderStage};
    use mali_g68::compiler::optimize::{optimize_shader, OptLevel};
    use mali_g68::compiler::emulator_pass::{detect_emulator_pattern, EmulatorPattern};
    use mali_g68::compiler::valhall::ValhallCompiler;
    use mali_g68::cmd::builder::CommandBufferBuilder;
    use mali_g68::cmd::draw::DrawInfo;
    use mali_g68::emulator::cache::PipelineCache;
    use mali_g68::emulator::async_compute::{AsyncComputeManager, TextureDecodeRequest, CompressedFormat};
    use mali_g68::vulkan::instance::{VkInstance, VkInstanceCreateInfo, VkApplicationInfo};
    use mali_g68::DriverConfig;

    #[test]
    fn test_gpu_detection() {
        let gpu = GpuInfo::mali_g68_mp5();
        assert_eq!(gpu.gpu_id, 0x9080);
        assert_eq!(gpu.num_shader_cores, 5);
    }

    #[test]
    fn test_csf_init() {
        let mut fw = CsfFirmware::new();
        fw.init().unwrap();
        assert!(fw.is_running());
    }

    #[test]
    fn test_shader_pipeline() {
        let mut shader = NirShader::new(ShaderStage::Fragment);
        shader.uses_textures = true;
        optimize_shader(&mut shader, OptLevel::Aggressive);
        assert!(shader.optimized);
        let pattern = detect_emulator_pattern(&shader);
        assert_eq!(pattern, EmulatorPattern::FragmentTexturing);
        let mut compiler = ValhallCompiler::new();
        let compiled = compiler.compile(&shader).unwrap();
        assert_eq!(compiled.stage, ShaderStage::Fragment);
    }

    #[test]
    fn test_command_buffer() {
        let mut cmd = CommandBufferBuilder::new(1);
        cmd.begin();
        cmd.begin_render_pass(640, 480, 1, false, false);
        cmd.draw(&DrawInfo::default());
        cmd.end_render_pass();
        cmd.end();
        assert_eq!(cmd.draw_count(), 1);
    }

    #[test]
    fn test_pipeline_cache() {
        let cache = PipelineCache::new();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_async_compute() {
        let mut manager = AsyncComputeManager::new();
        let request = TextureDecodeRequest {
            format: CompressedFormat::BC3,
            src_addr: 0x1000, src_size: 65536,
            dst_addr: 0x2000,
            width: 256, height: 256, depth: 1, mip_levels: 1,
        };
        let handle = manager.submit_decode(&request).unwrap();
        manager.complete_decode(&handle);
    }

    #[test]
    fn test_vulkan_instance() {
        let create_info = VkInstanceCreateInfo {
            app_info: VkApplicationInfo::default(),
            enabled_extensions: vec!["VK_KHR_surface".to_string()],
            enabled_layers: vec![],
            debug_enabled: false,
        };
        let instance = VkInstance::create(&create_info).unwrap();
        let devices = instance.enumerate_physical_devices().unwrap();
        assert_eq!(devices.len(), 1);
    }

    #[test]
    fn test_driver_config() {
        let config = DriverConfig::emulator_optimized();
        assert_eq!(config.emulator_opt_level, 3);
    }
}