//! Probe: list the GPU adapters wgpu can see on this machine, and which one the
//! default power preference would pick. Run: cargo run -p numinous-gpu --example info

fn main() {
    let instance = wgpu::Instance::default();

    println!("Adapters visible on this machine:");
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    for adapter in adapters {
        let info = adapter.get_info();
        println!(
            "  {:?} | {} | {:?} | driver: {}",
            info.backend, info.name, info.device_type, info.driver
        );
    }

    let chosen = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }));
    match chosen {
        Ok(adapter) => {
            let info = adapter.get_info();
            println!("\nDefault choice: {:?} | {}", info.backend, info.name);
        }
        Err(e) => println!("\nNo adapter chosen: {e:?}"),
    }
}
