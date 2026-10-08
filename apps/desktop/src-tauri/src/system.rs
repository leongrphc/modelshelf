use serde_json::{json, Value};

pub fn hardware() -> Value {
    let system = sysinfo::System::new_all();
    json!({
        "os": sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
        "cpu": system.cpus().first().map(|cpu| cpu.brand()).unwrap_or("Unknown"),
        "ram": system.total_memory(),
        "gpu": gpu()
    })
}

#[cfg(windows)]
fn gpu() -> String {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
    // DXGI only reads adapter descriptors. No driver-specific tools or commands run.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return "Unknown".into();
        };
        let mut adapters = Vec::new();
        let mut index = 0;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            if let Ok(desc) = adapter.GetDesc1() {
                let end = desc
                    .Description
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(desc.Description.len());
                let name = String::from_utf16_lossy(&desc.Description[..end]);
                adapters.push(format!(
                    "{} ({} MiB dedicated memory)",
                    name,
                    desc.DedicatedVideoMemory / 1024 / 1024
                ));
            }
            index += 1;
        }
        if adapters.is_empty() {
            "Unknown".into()
        } else {
            adapters.join("; ")
        }
    }
}

#[cfg(not(windows))]
fn gpu() -> String {
    "Unknown — GPU detection currently available on Windows".into()
}
