use opencl3::platform::get_platforms;
use opencl3::device::CL_DEVICE_TYPE_GPU;

fn main() {
    println!("Testing OpenCL GPU detection...\n");
    
    match get_platforms() {
        Ok(platforms) => {
            println!("Found {} OpenCL platform(s)", platforms.len());
            for (i, platform) in platforms.iter().enumerate() {
                let name = platform.name().unwrap_or_else(|_| "Unknown".to_string());
                let vendor = platform.vendor().unwrap_or_else(|_| "Unknown".to_string());
                let version = platform.version().unwrap_or_else(|_| "Unknown".to_string());
                
                println!("\nPlatform #{}: {}", i, name);
                println!("  Vendor: {}", vendor);
                println!("  Version: {}", version);
                
                match platform.get_devices(CL_DEVICE_TYPE_GPU) {
                    Ok(devices) => {
                        println!("  GPU devices: {}", devices.len());
                        for (j, device_id) in devices.iter().enumerate() {
                            if let Ok(device) = opencl3::device::Device::new(*device_id) {
                                let dev_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                                println!("    GPU #{}: {}", j, dev_name);
                            }
                        }
                    }
                    Err(e) => println!("  No GPU devices: {:?}", e),
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to get OpenCL platforms: {:?}", e);
        }
    }
}
