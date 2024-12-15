use std::process::Command;
use tracing::info;
use crate::adb::server::actions::models::action_config::ActionConfig;
use crate::constants::{CONNECT_EVENT,DISCONNECT_EVENT};

pub fn execute_action(config: &ActionConfig, serial: &str, event: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing {} actions for device {}", event, serial);

    match event {
        CONNECT_EVENT => {
            for action in &config.global.connect {
                execute_command(&action.cmd, serial)
                    .map_err(|e| format!("Failed to execute global connect action {}: {}", action.id, e))?;
            }
        }
        DISCONNECT_EVENT => {
            for action in &config.global.disconnect {
                execute_command(&action.cmd, serial)
                    .map_err(|e| format!("Failed to execute global disconnect action {}: {}", action.id, e))?;
            }
        }
        _ => {
            return Err(format!("Unknown event type: {}", event).into());
        }
    }

    if let Some(device_actions) = config.devices.get(serial) {
        match event {
            CONNECT_EVENT => {
                if let Some(actions) = &device_actions.connect {
                    for action in actions {
                        execute_command(&action.cmd, serial)
                            .map_err(|e| format!("Failed to execute device connect action {}: {}", action.id, e))?;
                    }
                }
            }
            DISCONNECT_EVENT => {
                if let Some(actions) = &device_actions.disconnect {
                    for action in actions {
                        execute_command(&action.cmd, serial)
                            .map_err(|e| format!("Failed to execute device disconnect action {}: {}", action.id, e))?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn execute_command(cmd: &str, serial: &str) -> Result<(), String> {
    let command = cmd.replace("{serial}", serial);
    info!("Executing command: {}", command);

    #[cfg(windows)]
        let output = Command::new("cmd")
        .args(["/C", &command])
        .output()
        .map_err(|e| format!("Failed to execute command on Windows: {}", e))?;

    #[cfg(not(windows))]
        let output = Command::new("sh")
        .args(["-c", &command])
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command failed: {}", error));
    }

    Ok(())
}