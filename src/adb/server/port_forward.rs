use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tracing::{info, warn};
use crate::adb::errors::adb_server_error::AdbServerError;
use crate::adb::io::socket::{send_fail_with_response, send_ok_with_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::models::adb_port_forward::AdbPortForward;
use crate::adb::models::adb_port_forward_info::AdbPortForwardInfo;
use crate::adb::server::server::AdbServer;
use crate::constants::{HOST_FORWARD_COMMAND, HOST_KILL_FORWARD_COMMAND, NO_REBIND_PORT_PREFIX};

impl AdbServer {

    const PORT_FORWARD_STOP_WAIT_MS: u64 = 200;
    const PORT_FORWARD_STOP_MAX_ATTEMPTS: u8 = 3;
    const PORT_FORWARD_READ_TIME_OUT_SECONDS: f64 = 0.4;

    pub async fn handle_port_forward_command_set(socket: &mut TcpStream, command: String, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let forward_params = command[HOST_FORWARD_COMMAND.len()..].to_string();

        let Some(adb_device) = chosen_adb_device else {
            return send_fail_with_response(socket, Some(AdbServerError::NoTransportSelected().to_string())).await;
        };

        let (no_rebind, params) = if forward_params.starts_with(NO_REBIND_PORT_PREFIX) {
            (true, forward_params[NO_REBIND_PORT_PREFIX.len()..].to_string())
        } else {
            (false, forward_params.clone())
        };

        let parts: Vec<&str> = params.split(';').collect();
        if parts.len() != 2 {
            warn!("Invalid port forward specification received");
            return send_fail_with_response(socket, Some("Invalid forward specification".to_string())).await;
        }

        let adb_port_forward_info = match AdbPortForwardInfo::from_string(&params) {
            Ok(info) => info,
            Err(err) => {
                warn!("Failed to parse port forward info: {}", err);
                return send_fail_with_response(socket, Some(err.to_string())).await;
            }
        };

        let local_port = adb_port_forward_info.local_with_type();
        if adb_device.has_port_forward(&local_port) {
            if no_rebind {
                warn!("Port forward already exists for {}, no-rebind specified", &local_port);
                return send_fail_with_response(socket, Some(format!("Port forward already exists for local port {}. Cannot rebind due to no-rebind option", &local_port))).await;
            }
            if let Some(port_forward_ref) = adb_device.get_port_forward(&adb_port_forward_info.local_with_type()) {
                info!("Stopping existing port forward for {}", &local_port);
                if let Err(err) = Self::stop_port_forward_spawn(port_forward_ref.value()).await {
                    warn!("Failed to stop existing port forward: {}", err);
                    return send_fail_with_response(socket, Some(err.to_string())).await;
                }
            }
        }

        let adb_device_clone = Arc::clone(&adb_device);
        let adb_port_forward_info_clone = adb_port_forward_info.clone();

        let (port_forward_result_sender, port_forward_result_receiver) = oneshot::channel();
        let adb_forward_task = tokio::spawn(async move {
            adb_device_clone.adb_device_connection().adb_port_forward_set(adb_port_forward_info_clone, port_forward_result_sender, Some(Self::PORT_FORWARD_READ_TIME_OUT_SECONDS))
                .await;
        });

        match port_forward_result_receiver.await {
            Ok(Ok(())) => {
                info!("Port forward established for {}", &local_port);
                let adb_port_forward = AdbPortForward::new(adb_port_forward_info.clone(), adb_forward_task);
                adb_device.insert_port_forward(adb_port_forward_info.local_with_type(), adb_port_forward);
                send_ok_with_response(socket, None).await
            }
            Ok(Err(err)) => {
                warn!("Failed to establish port forward: {}", err);
                send_fail_with_response(socket, Some(err.to_string())).await
            }
            Err(_) => {
                warn!("Unexpected error during port forward setup");
                send_fail_with_response(socket, Some("Unexpected error occurred during port forwarding".to_string())).await
            }
        }
    }

    pub async fn port_forward_remove(socket: &mut TcpStream, command: String, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let local_port = &command[HOST_KILL_FORWARD_COMMAND.len()..];
        let adb_device = chosen_adb_device.ok_or_else(|| AdbServerError::NoTransportSelected())?;

        let port_forward = match adb_device.get_port_forward(local_port) {
            Some(pf) => pf,
            None => {
                warn!("Port forward not found for {}", local_port);
                return send_fail_with_response(socket, Some(format!("listener '{}' not found", local_port))).await;
            }
        };
        let stop_result = Self::stop_port_forward_spawn(port_forward.value()).await;
        drop(port_forward);

        match stop_result {
            Ok(_) => {
                info!("Port forward removed for {}", local_port);
                adb_device.remove_port_forward(local_port);
                send_ok_with_response(socket, None).await
            }
            Err(err) => send_fail_with_response(socket, Some(err.to_string())).await,
        }
    }

    pub async fn port_forward_list(socket: &mut TcpStream, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let adb_device = chosen_adb_device.ok_or_else(|| AdbServerError::NoTransportSelected())?;
        let mut response = String::new();

        for entry in adb_device.adb_ports_forward_hs().iter() {
            let port_forward = entry.value();
            let forward_info = port_forward.get_info();
            response.push_str(&format!("{} {}\n",
                                       forward_info.local_with_type(),
                                       forward_info.remote_with_type()
            ));
        }

        send_ok_with_response(socket, Some(response)).await
    }

    pub async fn port_forward_remove_all(socket: &mut TcpStream, chosen_adb_device: Option<Arc<AdbDevice>>) -> Result<(), AdbServerError> {
        let mut ports_to_remove = Vec::new();
        let adb_device = chosen_adb_device.ok_or_else(|| AdbServerError::NoTransportSelected())?;

        for entry in adb_device.adb_ports_forward_hs().iter() {
            let key = entry.key().clone();
            info!("Stopping port forward for {}", key);
            let stop_result = Self::stop_port_forward_spawn(entry.value()).await;
            match stop_result {
                Ok(_) => {
                    ports_to_remove.push(key);
                }
                Err(err) => {
                    warn!("Failed to stop port forward for {}: {}", key, err);
                    return send_fail_with_response(socket, Some(err.to_string())).await;
                }
            }
        }

        for port in ports_to_remove {
            adb_device.remove_port_forward(&port);
        }

        info!("All port forwards removed");
        send_ok_with_response(socket, None).await
    }

    async fn stop_port_forward_spawn(adb_port_forward: &AdbPortForward) -> Result<(), AdbServerError> {
        adb_port_forward.stop();
        for attempt in 0..Self::PORT_FORWARD_STOP_MAX_ATTEMPTS {
            if !adb_port_forward.is_running() {
                return Ok(());
            }
            if attempt == Self::PORT_FORWARD_STOP_MAX_ATTEMPTS - 1 {
                warn!("Port forward stop timeout after {} attempts", Self::PORT_FORWARD_STOP_MAX_ATTEMPTS);
            }
            tokio::time::sleep(Duration::from_millis(Self::PORT_FORWARD_STOP_WAIT_MS)).await;
        }

        if adb_port_forward.is_running() {
            return Err(AdbServerError::UnexpectedError("Failed to stop port forward".to_string()));
        }
        Ok(())
    }
}