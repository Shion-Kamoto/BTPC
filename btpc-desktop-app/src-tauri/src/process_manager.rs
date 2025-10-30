//! Process Manager - Handles long-running background processes
//!
//! This module provides robust process management for the desktop app,
//! ensuring processes survive page navigation and are properly cleaned up.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub started_at: String,
    pub status: ProcessStatus,
    pub restart_count: u32,
    pub command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProcessStatus {
    Starting,
    Running,
    Stopped,
    Crashed,
}

pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
    auto_restart: bool,
}

impl ProcessManager {
    pub fn new(auto_restart: bool) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            auto_restart,
        }
    }

    /// Scan system for existing processes and adopt them
    /// This allows the app to detect and manage processes started externally
    pub fn scan_and_adopt(&self, process_names: Vec<(&str, &str)>) -> Vec<String> {
        let mut adopted = Vec::new();

        for (name, binary_name) in process_names {
            if let Some(pid) = self.find_process_by_name(binary_name) {
                // Only adopt if not already managed
                let already_managed = {
                    let processes = self.processes.lock().unwrap();
                    processes.contains_key(name)
                };

                if !already_managed {
                    match self.adopt_existing_process(name.to_string(), binary_name.to_string(), pid) {
                        Ok(_) => {
                            println!("✅ Adopted existing {} process (PID: {})", name, pid);
                            adopted.push(format!("{} (PID: {})", name, pid));
                        }
                        Err(e) => {
                            eprintln!("⚠️ Failed to adopt {} process: {}", name, e);
                        }
                    }
                }
            }
        }

        adopted
    }

    /// Find a process by binary name and return its PID
    #[cfg(unix)]
    fn find_process_by_name(&self, binary_name: &str) -> Option<u32> {
        let output = std::process::Command::new("pgrep")
            .args(["-f", binary_name])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Get the first PID (oldest process)
            stdout.lines()
                .next()
                .and_then(|line| line.trim().parse::<u32>().ok())
        } else {
            None
        }
    }

    #[cfg(windows)]
    fn find_process_by_name(&self, binary_name: &str) -> Option<u32> {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}.exe", binary_name), "/FO", "CSV", "/NH"])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse CSV output: "btpc_node.exe","1234","Console","1","12,345 K"
            stdout.lines()
                .next()
                .and_then(|line| {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 2 {
                        parts[1].trim_matches('"').parse::<u32>().ok()
                    } else {
                        None
                    }
                })
        } else {
            None
        }
    }

    /// Adopt an existing process into the manager
    pub fn adopt_existing_process(
        &self,
        name: String,
        binary_name: String,
        pid: u32,
    ) -> Result<ProcessInfo, String> {
        // Verify the process is actually running
        if !self.check_pid_running(pid) {
            return Err(format!("Process with PID {} is not running", pid));
        }

        let info = ProcessInfo {
            pid,
            name: name.clone(),
            started_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            status: ProcessStatus::Running,
            restart_count: 0,
            command: format!("{} (adopted)", binary_name),
        };

        // Store process info
        {
            let mut processes = self.processes.lock()
                .map_err(|e| format!("Failed to acquire process lock: {}", e))?;
            processes.insert(name.clone(), info.clone());
        }

        Ok(info)
    }

    /// Start a process as detached (survives parent lifecycle)
    pub fn start_detached(
        &self,
        name: String,
        command: String,
        args: Vec<String>,
        stdout_path: Option<PathBuf>,
        stderr_path: Option<PathBuf>,
    ) -> Result<ProcessInfo, String> {
        let mut cmd = Command::new(&command);
        cmd.args(&args);

        // Critical: Detach from parent process
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    // Create new session (detach from terminal)
                    // SAFETY: setsid() is safe to call here, but we must check return value
                    let result = libc::setsid();
                    if result == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        // Redirect output
        if let Some(stdout) = stdout_path {
            let file = std::fs::File::create(stdout)
                .map_err(|e| format!("Failed to create stdout file: {}", e))?;
            cmd.stdout(Stdio::from(file));
        } else {
            cmd.stdout(Stdio::null());
        }

        if let Some(stderr) = stderr_path {
            let file = std::fs::File::create(stderr)
                .map_err(|e| format!("Failed to create stderr file: {}", e))?;
            cmd.stderr(Stdio::from(file));
        } else {
            cmd.stderr(Stdio::null());
        }

        cmd.stdin(Stdio::null());

        // Spawn and immediately detach
        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        let pid = child.id();

        // FIXED: Instead of mem::forget (memory leak), spawn a thread to reap the zombie
        // This thread will call wait() in a non-blocking way and exit immediately
        std::thread::spawn(move || {
            // Attempt to wait for the process, but don't block
            // If the process is still running, this will eventually reap it when it exits
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                // Wait in non-blocking mode for this specific child
                // This prevents zombie accumulation
                let _ = child.wait();
            }
            #[cfg(windows)]
            {
                let _ = child.wait();
            }
        });

        let info = ProcessInfo {
            pid,
            name: name.clone(),
            started_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            status: ProcessStatus::Running,
            restart_count: 0,
            command: format!("{} {}", command, args.join(" ")),
        };

        // Store process info
        {
            let mut processes = self.processes.lock()
                .map_err(|e| format!("Failed to acquire process lock: {}", e))?;
            processes.insert(name.clone(), info.clone());
        }

        Ok(info)
    }

    /// Check if process is actually running (not just in our map)
    pub fn is_running(&self, name: &str) -> bool {
        let processes = match self.processes.lock() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to acquire process lock in is_running: {}", e);
                return false;
            }
        };

        if let Some(info) = processes.get(name) {
            self.check_pid_running(info.pid)
        } else {
            false
        }
    }

    /// Stop a process gracefully (SIGTERM) with timeout and fallback to SIGKILL
    pub fn stop(&self, name: &str) -> Result<(), String> {
        let pid = {
            let processes = self.processes.lock()
                .map_err(|e| format!("Failed to acquire process lock: {}", e))?;
            if let Some(info) = processes.get(name) {
                info.pid
            } else {
                return Err(format!("Process '{}' not found", name));
            }
        };

        // Send SIGTERM for graceful shutdown
        #[cfg(unix)]
        {
            std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;
        }

        #[cfg(windows)]
        {
            std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T"])
                .status()
                .map_err(|e| format!("Failed to send taskkill: {}", e))?;
        }

        // Wait up to 5 seconds for graceful shutdown
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();
        let check_interval = std::time::Duration::from_millis(100);

        while start.elapsed() < timeout {
            if !self.check_pid_running(pid) {
                // Process stopped successfully
                let mut processes = self.processes.lock()
                    .map_err(|e| format!("Failed to acquire process lock: {}", e))?;
                if let Some(info) = processes.get_mut(name) {
                    info.status = ProcessStatus::Stopped;
                }
                return Ok(());
            }
            std::thread::sleep(check_interval);
        }

        // Graceful shutdown timed out, force kill
        println!("⚠️ Process {} did not stop gracefully, force killing...", name);

        #[cfg(unix)]
        {
            std::process::Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status()
                .map_err(|e| format!("Failed to send SIGKILL: {}", e))?;
        }

        #[cfg(windows)]
        {
            std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F", "/T"])
                .status()
                .map_err(|e| format!("Failed to force kill: {}", e))?;
        }

        // Wait another second for force kill to complete
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Update status
        let mut processes = self.processes.lock()
            .map_err(|e| format!("Failed to acquire process lock: {}", e))?;
        if let Some(info) = processes.get_mut(name) {
            info.status = ProcessStatus::Stopped;
        }

        Ok(())
    }

    /// Force kill a process (SIGKILL)
    pub fn kill(&self, name: &str) -> Result<(), String> {
        let mut processes = self.processes.lock()
            .map_err(|e| format!("Failed to acquire process lock: {}", e))?;

        if let Some(info) = processes.get_mut(name) {
            #[cfg(unix)]
            {
                std::process::Command::new("kill")
                    .args(["-KILL", &info.pid.to_string()])
                    .status()
                    .map_err(|e| format!("Failed to kill process: {}", e))?;
            }

            #[cfg(windows)]
            {
                std::process::Command::new("taskkill")
                    .args(["/PID", &info.pid.to_string(), "/F", "/T"])
                    .status()
                    .map_err(|e| format!("Failed to kill process: {}", e))?;
            }

            info.status = ProcessStatus::Stopped;
            Ok(())
        } else {
            Err(format!("Process '{}' not found", name))
        }
    }

    /// Get process info
    pub fn get_info(&self, name: &str) -> Option<ProcessInfo> {
        let processes = match self.processes.lock() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to acquire process lock in get_info: {}", e);
                return None;
            }
        };
        processes.get(name).cloned()
    }

    /// Get all processes
    pub fn list_all(&self) -> Vec<ProcessInfo> {
        let processes = match self.processes.lock() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to acquire process lock in list_all: {}", e);
                return Vec::new();
            }
        };
        processes.values().cloned().collect()
    }

    /// Health check - update status of all processes
    pub fn health_check(&self) {
        let mut processes = match self.processes.lock() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to acquire process lock in health_check: {}", e);
                return;
            }
        };

        for (_name, info) in processes.iter_mut() {
            let running = self.check_pid_running(info.pid);

            if !running && info.status == ProcessStatus::Running {
                info.status = ProcessStatus::Crashed;
            }
        }
    }

    /// Remove a process from tracking
    pub fn remove(&self, name: &str) {
        let mut processes = match self.processes.lock() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to acquire process lock in remove: {}", e);
                return;
            }
        };
        processes.remove(name);
    }

    /// Stop all processes
    pub fn stop_all(&self) {
        let processes = match self.processes.lock() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to acquire process lock in stop_all: {}", e);
                return;
            }
        };
        let names: Vec<String> = processes.keys().cloned().collect();
        drop(processes);

        for name in names {
            let _ = self.stop(&name);
        }
    }

    #[cfg(unix)]
    fn check_pid_running(&self, pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn check_pid_running(&self, pid: u32) -> bool {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        // Clean up all processes when manager is dropped
        self.stop_all();
    }
}