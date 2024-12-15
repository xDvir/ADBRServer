use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tracing::{info, warn};
use crate::adb::errors::adb_server_error::AdbServerError;
use crate::adb::io::socket::{send_fail_with_response, send_ok_with_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::models::adb_port_reverse::AdbPortReverse;
use crate::adb::models::adb_port_reverse_info::AdbPortReverseInfo;
use crate::adb::server::server::AdbServer;
use crate::constants::{NO_REBIND_PORT_PREFIX, REVERSE_FORWARD_COMMAND, REVERSE_KILL_FORWARD_COMMAND};

impl AdbServer {

    const PORT_REVERSE_STOP_WAIT_MS: u64 = 200;
    const PORT_REVERSE_STOP_MAX_ATTEMPTS: u8 = 3;
    const PORT_REVERSE_READ_TIME_OUT_SECONDS: f64 = 1.5;

    pub async fn handle_port_reverse_command_set(socket: &mut TcpStream, command: String, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let reverse_params = command[REVERSE_FORWARD_COMMAND.len()..].to_string();

        let adb_device = chosen_adb_device.ok_or_else(|| AdbServerError::NoTransportSelected())?;

        let (no_rebind, params) = if reverse_params.starts_with(NO_REBIND_PORT_PREFIX) {
            (true, reverse_params[NO_REBIND_PORT_PREFIX.len()..].to_string())
        } else {
            (false, reverse_params.clone())
        };

        let parts: Vec<&str> = params.split(';').collect();
        if parts.len() != 2 {
            warn!("Invalid port reverse specification received");
            return send_fail_with_response(socket, Some("Invalid reverse specification".to_string())).await;
        }

        let adb_port_reverse_info = match AdbPortReverseInfo::from_string(&params) {
            Ok(info) => info,
            Err(err) => {
                warn!("Failed to parse port reverse info: {}", err);
                return send_fail_with_response(socket, Some(err.to_string())).await;
            }
        };

        let remote_port = adb_port_reverse_info.device_with_type();
        if adb_device.has_port_reverse(&remote_port) {
            if no_rebind {
                warn!("Port reverse already exists for {}, no-rebind specified", &remote_port);
                return send_fail_with_response(socket, Some(format!("Port reverse already exists for remote port {}. Cannot rebind due to no-rebind option", &remote_port))).await;
            }
            if let Some(port_reverse_ref) = adb_device.get_port_reverse(&remote_port) {
                info!("Stopping existing port reverse for {}", &remote_port);
                if let Err(err) = Self::stop_port_reverse_spawn(port_reverse_ref.value()).await {
                    warn!("Failed to stop existing port reverse: {}", err);
                    return send_fail_with_response(socket, Some(err.to_string())).await;
                }
            }
        }

        send_ok_with_response(socket, None).await?;

        let adb_device_clone = Arc::clone(&adb_device);
        let adb_port_reverse_info_clone = adb_port_reverse_info.clone();

        let (port_reverse_result_sender, port_reverse_result_receiver) = oneshot::channel();
        let adb_reverse_task = tokio::spawn(async move {
            adb_device_clone.adb_device_connection().clone().adb_port_reverse_set(adb_port_reverse_info_clone, port_reverse_result_sender, Some(Self::PORT_REVERSE_READ_TIME_OUT_SECONDS)).await;
        });

        match port_reverse_result_receiver.await {
            Ok(Ok(())) => {
                info!("Port reverse established for {}", &remote_port);
                let adb_port_reverse = AdbPortReverse::new(adb_port_reverse_info.clone(), adb_reverse_task);
                adb_device.insert_port_reverse(remote_port, adb_port_reverse);
                send_ok_with_response(socket, Some(adb_port_reverse_info.device_with_type())).await
            }
            Ok(Err(err)) => {
                warn!("Failed to establish port reverse: {}", err);
                send_fail_with_response(socket, Some(err.to_string())).await
            }
            Err(_) => {
                warn!("Unexpected error during port reverse setup");
                send_fail_with_response(socket, Some("Unexpected error occurred during port reversing".to_string())).await
            }
        }
    }

    pub async fn port_reverse_remove(socket: &mut TcpStream, command: String, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let remote_port = &command[REVERSE_KILL_FORWARD_COMMAND.len()..];
        let adb_device = chosen_adb_device.ok_or_else(|| AdbServerError::NoTransportSelected())?;

        let port_reverse = match adb_device.get_port_reverse(remote_port) {
            Some(pr) => pr,
            None => {
                warn!("Port reverse not found for {}", remote_port);
                return send_fail_with_response(socket, Some(format!("listener '{}' not found", remote_port))).await;
            }
        };

        let stop_result = Self::stop_port_reverse_spawn(port_reverse.value()).await;
        drop(port_reverse);

        match stop_result {
            Ok(_) => {
                info!("Port reverse removed for {}", remote_port);
                adb_device.remove_port_reverse(remote_port);
                send_ok_with_response(socket, None).await
            }
            Err(err) => {
                warn!("Failed to remove port reverse: {}", err);
                send_fail_with_response(socket, Some(err.to_string())).await
            }
        }
    }

    pub async fn port_reverse_remove_all(socket: &mut TcpStream, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let mut ports_to_remove = Vec::new();
        let adb_device = chosen_adb_device.ok_or_else(|| AdbServerError::NoTransportSelected())?;

        for entry in adb_device.adb_ports_reverse_hs().iter() {
            let key = entry.key().clone();
            info!("Stopping port reverse for {}", key);
            let stop_result = Self::stop_port_reverse_spawn(entry.value()).await;
            match stop_result {
                Ok(_) => ports_to_remove.push(key),
                Err(err) => {
                    warn!("Failed to stop port reverse for {}: {}", key, err);
                    return send_fail_with_response(socket, Some(err.to_string())).await;
                }
            }
        }

        for port in ports_to_remove {
            adb_device.remove_port_reverse(&port);
        }

        info!("All port reverses removed");
        send_ok_with_response(socket, None).await
    }

    pub async fn port_reverse_list(socket: &mut TcpStream, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let adb_device = chosen_adb_device.ok_or_else(|| AdbServerError::NoTransportSelected())?;
        let mut response = String::new();

        for entry in adb_device.adb_ports_reverse_hs().iter() {
            let port_reverse = entry.value();
            let reverse_info = port_reverse.get_info();
            response.push_str(&format!("{} {}\n", reverse_info.device_with_type(), reverse_info.host_with_type()));
        }

        send_ok_with_response(socket, Some(response)).await
    }

    async fn stop_port_reverse_spawn(adb_port_reverse: &AdbPortReverse) -> Result<(), AdbServerError> {
        adb_port_reverse.stop();

        for attempt in 0..Self::PORT_REVERSE_STOP_MAX_ATTEMPTS {
            if !adb_port_reverse.is_running() {
                return Ok(());
            }
            if attempt == Self::PORT_REVERSE_STOP_MAX_ATTEMPTS - 1 {
                warn!("Port reverse stop timeout after {} attempts", Self::PORT_REVERSE_STOP_MAX_ATTEMPTS);
            }
            tokio::time::sleep(Duration::from_millis(Self::PORT_REVERSE_STOP_WAIT_MS)).await;
        }

        if adb_port_reverse.is_running() {
            return Err(AdbServerError::UnexpectedError("Failed to stop port reverse".to_string()));
        }

        Ok(())
    }
}