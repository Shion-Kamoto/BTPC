// Quick test to diagnose OpenCL kernel compilation issue
use opencl3::context::Context;
use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};
use opencl3::platform::get_platforms;
use opencl3::program::Program;

const KERNEL_SOURCE: &str = include_str!("sha512_kernel.cl");

fn main() {
    println!("=== OpenCL Diagnostic Test ===\n");

    // Get platforms
    let platforms = match get_platforms() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to get platforms: {}", e);
            return;
        }
    };

    println!("✅ Found {} platform(s)", platforms.len());

    // Get GPU devices
    let device_ids = match get_all_devices(CL_DEVICE_TYPE_GPU) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ Failed to get GPU devices: {}", e);
            return;
        }
    };

    if device_ids.is_empty() {
        eprintln!("❌ No GPU devices found");
        return;
    }

    println!("✅ Found {} GPU device(s)", device_ids.len());

    // Test with first device
    let device = Device::new(device_ids[0]);
    let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
    let device_vendor = device.vendor().unwrap_or_else(|_| "Unknown".to_string());
    let opencl_version = device.opencl_c_version().unwrap_or_else(|_| "Unknown".to_string());

    println!("\n📋 Device Info:");
    println!("  Name: {}", device_name);
    println!("  Vendor: {}", device_vendor);
    println!("  OpenCL C Version: {}", opencl_version);

    // Create context
    let context = match Context::from_device(&device) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to create context: {}", e);
            return;
        }
    };

    println!("\n✅ Created OpenCL context");

    // Test 1: Try with -cl-std=CL1.0 (bypasses Mesa's broken libclc headers)
    println!("\n🔨 Test 1: Building kernel with -cl-std=CL1.0...");
    match Program::create_and_build_from_source(&context, KERNEL_SOURCE, "-cl-std=CL1.0 -w") {
        Ok(_) => {
            println!("✅ SUCCESS: Kernel compiled with CL1.0 standard!");
            println!("✅ GPU mining READY - Mesa libclc workaround successful");
            return;
        }
        Err(e) => {
            println!("❌ Failed with CL1.0: {}", e);
        }
    }

    // Test 2: Try creating program first to get build log
    println!("\n🔨 Test 2: Creating program and building separately...");
    match Program::create_from_source(&context, KERNEL_SOURCE) {
        Ok(mut prog) => {
            println!("✅ Program created successfully");

            match prog.build(&[device.id()], "") {
                Ok(_) => {
                    println!("✅ SUCCESS: Build succeeded on second attempt!");
                }
                Err(build_err) => {
                    println!("❌ Build failed: {}", build_err);

                    // Get build log
                    match prog.get_build_log(device.id()) {
                        Ok(log) => {
                            println!("\n📋 BUILD LOG:");
                            println!("{}", log);
                            if log.trim().is_empty() {
                                println!("⚠️ Build log is EMPTY (this is the bug!)");
                            }
                        }
                        Err(log_err) => {
                            println!("❌ Could not get build log: {}", log_err);
                        }
                    }
                }
            }
        }
        Err(create_err) => {
            println!("❌ Failed to create program: {}", create_err);
        }
    }

    // Test 3: Try with OpenCL 1.2 compatibility flag
    println!("\n🔨 Test 3: Building with -cl-std=CL1.2...");
    match Program::create_from_source(&context, KERNEL_SOURCE) {
        Ok(mut prog) => {
            match prog.build(&[device.id()], "-cl-std=CL1.2") {
                Ok(_) => {
                    println!("✅ SUCCESS: Kernel compiled with CL1.2 flag!");
                }
                Err(e) => {
                    println!("❌ Failed: {}", e);
                    if let Ok(log) = prog.get_build_log(device.id()) {
                        println!("Build log: {}", log);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create program: {}", e);
        }
    }

    // Test 4: Try with relaxed math
    println!("\n🔨 Test 4: Building with -cl-fast-relaxed-math...");
    match Program::create_from_source(&context, KERNEL_SOURCE) {
        Ok(mut prog) => {
            match prog.build(&[device.id()], "-cl-fast-relaxed-math") {
                Ok(_) => {
                    println!("✅ SUCCESS: Kernel compiled with relaxed math!");
                }
                Err(e) => {
                    println!("❌ Failed: {}", e);
                    if let Ok(log) = prog.get_build_log(device.id()) {
                        println!("Build log: {}", log);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create program: {}", e);
        }
    }

    // Test 5: Try with -cl-no-stdinc to skip broken system headers
    println!("\n🔨 Test 5: Building with -cl-no-stdinc (skip system headers)...");
    match Program::create_from_source(&context, KERNEL_SOURCE) {
        Ok(mut prog) => {
            match prog.build(&[device.id()], "-cl-no-stdinc") {
                Ok(_) => {
                    println!("✅ SUCCESS: Kernel compiled by skipping system headers!");
                    println!("This is the SOLUTION to the bug!");
                }
                Err(e) => {
                    println!("❌ Failed: {}", e);
                    if let Ok(log) = prog.get_build_log(device.id()) {
                        println!("Build log: {}", log);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create program: {}", e);
        }
    }
}
