use std::sync::{Arc, Mutex};
use std::thread::{spawn};
use std::time::Duration;
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::net::{TcpListener, TcpStream};
use crate::adb::connections::adb_connection::AdbConnection;
use futures::stream::{self, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::time;
use tracing::{error, info, warn};
use crate::adb::connections::adb_device_connection::{AdbDeviceConnection};
use crate::adb::enums::adb_device_status::AdbDeviceStatus;
use crate::adb::enums::adb_device_transport::AdbDeviceTransport;
use crate::adb::enums::adb_device_type::AdbDeviceType;
use crate::scanners::devices_scanner::DevicesScanner;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_io_error::AdbIoError;
use crate::adb::errors::adb_server_error::AdbServerError;
use crate::adb::io::socket::{read_request_from_socket, send_fail_with_response, send_full_response, send_ok_with_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::models::adb_task::AdbTask;
use crate::adb::server::actions::executor::execute_action;
use crate::adb::server::actions::models::action_config::ActionConfig;
use crate::constants::{EXIT_FAILURE, HOST_DEVICES_COMMAND, HOST_EMULATOR_ANY_COMMAND, HOST_TRANSPORT_ANY_COMMAND, HOST_TRANSPORT_COMMAND, HOST_USB_ANY_COMMAND, HOST_VERSION_COMMAND, SHELL_COMMAND, HOST_FORWARD_COMMAND, HOST_KILL_FORWARD_COMMAND, HOST_FORWARD_KILL_ALL_COMMAND, REBOOT_COMMAND, SYNC_COMMAND, SYNC_STAT_COMMAND_STR, SYNC_SEND_COMMAND_STR, SYNC_DATA_COMMAND_STR, SYNC_QUIT_COMMAND_STR, SYNC_RECV_COMMAND_STR, SYNC_DENT_COMMAND_STR, HOST_SERIALNO_COMMAND, HOST_GET_DEVPATH_COMMAND, ROOT_COMMAND, UNROOT_COMMAND, REMOUNT_COMMAND, ENABLE_VERITY_COMMAND, DISABLE_VERITY_COMMAND, HOST_FORWARD_LIST_COMMAND, HOST_GET_STATE_COMMAND, REVERSE_FORWARD_COMMAND, REVERSE_KILL_FORWARD_COMMAND, REVERSE_KILL_ALL_FORWARD_COMMAND, REVERSE_FORWARD_LIST_COMMAND, OKAY, ADB_SERVER_VERSION, DEFAULT_ADB_SERVER_PORT, CONNECT_EVENT, DISCONNECT_EVENT};
use crate::transport::enums::interface_type::InterfaceType;
use crate::transport::transport::Transport;



lazy_static! {
    pub static ref ADB_SERVER_INSTANCE: AdbServer = AdbServer::new();
}

pub struct AdbServer {
    pub adb_devices_hashmap: DashMap<String, Option<Arc<AdbDevice>>>,
    task_sender: Sender<AdbTask>,
    task_receiver: Arc<Mutex<Receiver<AdbTask>>>,
}

impl AdbServer {

    const MONITORING_INTERVAL_MS: u64 = 500;
    const DEVICE_AVAILABLE_VERIFY_TIME_SECONDS: u64 = 2;
    const DEVICE_NOT_AVAILABLE_RECONNECT_TIME_SECONDS: u64 = 20;
    const DEVICE_UNAUTHORIZED_RECONNECT_TIME_SECONDS: u64 = 5;
    const TASK_CHANNEL_SIZE: usize = 32;

    fn new() -> AdbServer {
        let (task_sender, task_receiver) = mpsc::channel(Self::TASK_CHANNEL_SIZE);
        AdbServer {
            adb_devices_hashmap: DashMap::new(),
            task_sender,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
        }
    }

    pub fn get_instance() -> &'static Self {
        &ADB_SERVER_INSTANCE
    }

    pub async fn init(&self, server_listen_address: String, server_port: Option<u16>) {
        let scanning_for_devices_task = spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                DevicesScanner::new().start_scanning(
                    InterfaceType::android_usb(),
                    Arc::new(|serial: String| Self::is_new_device(serial)),
                    Arc::new(|serial_number: String, transport: Box<dyn Transport>, interface_type: InterfaceType| Box::pin(Self::on_find_device(serial_number, transport, interface_type))),
                ).await;
            });
        });

        let monitor_connected_devices_task = spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                Self::monitor_connected_devices().await
            });
        });

        let start_listen_client_requests_task = spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                Self::start_listen_client_requests(server_listen_address.clone(), server_port).await;
            });
        });

        let process_incoming_tasks_task = spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                Self::process_incoming_tasks().await;
            });
        });

        tokio::spawn(async move {
            if let Err(err) = scanning_for_devices_task.join() {
                error!("Critical error: Scanning for new devices failed. Error: {:?}. Terminating server.", err);
                std::process::exit(EXIT_FAILURE);
            }

            if let Err(err) = monitor_connected_devices_task.join() {
                error!("Critical error: Failed to monitor connected devices. Error: {:?}. Terminating server.", err);
                std::process::exit(EXIT_FAILURE);
            }

            if let Err(err) = start_listen_client_requests_task.join() {
                error!("Critical error: Failed to listen to client requests. Error: {:?}. Terminating server.", err);
                std::process::exit(EXIT_FAILURE);
            }

            if let Err(err) = process_incoming_tasks_task.join() {
                error!("Critical error: Failed to process incoming tasks. Error: {:?}. Terminating server.", err);
                std::process::exit(EXIT_FAILURE);
            }
        });
    }

    async fn start_listen_client_requests(server_listen_address: String, server_port: Option<u16>) {
        let port = server_port.unwrap_or(DEFAULT_ADB_SERVER_PORT);
        let server_address = format!("{}:{}", server_listen_address, port);

        let listener = match TcpListener::bind(&server_address).await {
            Ok(listener) => {
                listener
            }
            Err(err) => {
                error!("Failed to bind to address {}:{}: {}", server_listen_address, port, err);
                std::process::exit(EXIT_FAILURE);
            }
        };

        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    tokio::spawn(async move {
                        let task = AdbTask::new(socket);
                        if ADB_SERVER_INSTANCE.task_sender.send(task).await.is_err() {
                            error!("Failed to accept client request");
                        }
                    });
                }
                Err(err) => {
                    error!("Failed to accept client request: {}", err);
                }
            }
        }
    }

    async fn process_incoming_tasks() {
        loop {
            let task = {
                let mut guard = match ADB_SERVER_INSTANCE.task_receiver.lock() {
                    Ok(guard) => guard,
                    Err(err) => {
                        error!("Failed to acquire task receiver lock: {:?}", err);
                        continue;
                    }
                };
                guard.recv().await
            };

            if let Some(task) = task {
                tokio::spawn(async move {
                    Self::execute_client_task(task).await;
                });
            } else {
                break;
            }
        }
    }

    async fn execute_client_task(mut task: AdbTask) {
        let socket = &mut task.socket;
        let mut chosen_adb_device: Option<Arc<AdbDevice>> = None;

        let mut request = match Self::read_next_client_request(socket).await {
            Ok(req) => req,
            Err(err) => {
                error!("Failed to read client request: {}", err);
                return;
            }
        };

        while !request.is_empty() {
            match &request {
                command if command.starts_with(HOST_TRANSPORT_ANY_COMMAND) => {
                    chosen_adb_device = Self::execute_transport_command(socket, AdbDeviceTransport::Any).await;
                    if chosen_adb_device.is_none() {
                        info!("No device found for transport any command");
                        break;
                    }
                }
                command if command.starts_with(HOST_EMULATOR_ANY_COMMAND) => {
                    chosen_adb_device = Self::execute_transport_command(socket, AdbDeviceTransport::EmulatorAny).await;
                    if chosen_adb_device.is_none() {
                        info!("No emulator device found");
                        break;
                    }
                }
                command if command.starts_with(HOST_USB_ANY_COMMAND) => {
                    chosen_adb_device = Self::execute_transport_command(socket, AdbDeviceTransport::UsbAny).await;
                    if chosen_adb_device.is_none() {
                        info!("No USB device found");
                        break;
                    }
                }
                command if command.starts_with(HOST_TRANSPORT_COMMAND) => {
                    let serial_number = &command[HOST_TRANSPORT_COMMAND.len()..];
                    chosen_adb_device = Self::execute_transport_command(socket, AdbDeviceTransport::usb(serial_number)).await;
                    if chosen_adb_device.is_none() {
                        info!("No device found for serial: {}", serial_number);
                        break;
                    }
                }
                command if command.starts_with(HOST_VERSION_COMMAND) => {
                    Self::send_version_response(socket).await;
                    break;
                }
                command if command.starts_with(HOST_GET_STATE_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            if let Err(err) = send_ok_with_response(socket, Some(adb_device.adb_device_status().to_string())).await {
                                warn!("Failed send device state {}", err);
                            }
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(HOST_DEVICES_COMMAND) => {
                    Self::adb_devices(socket).await;
                    break;
                }
                command if command.starts_with(REBOOT_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_reboot_command(adb_device, command, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command == ENABLE_VERITY_COMMAND => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_enable_verity_command(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command == DISABLE_VERITY_COMMAND => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_disable_verity_command(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(REMOUNT_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_remount_command(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(ROOT_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_root_command(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(UNROOT_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_unroot_command(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(HOST_SERIALNO_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_serialno_command(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(HOST_GET_DEVPATH_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_get_devpath(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(SHELL_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_shell_command(adb_device, socket, command.clone()).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(HOST_FORWARD_COMMAND) => {
                    if let Err(err) = Self::handle_port_forward_command_set(socket, command.clone(), chosen_adb_device).await {
                        error!("Error handling port forward command: {}", err);
                    }
                    break;
                }
                command if command.starts_with(HOST_KILL_FORWARD_COMMAND) => {
                    if let Err(err) = Self::port_forward_remove(socket, command.clone(), chosen_adb_device).await {
                        error!("Error removing port forward command: {}", err);
                    }
                    break;
                }
                command if command.starts_with(HOST_FORWARD_KILL_ALL_COMMAND) => {
                    if let Err(err) = Self::port_forward_remove_all(socket, chosen_adb_device).await {
                        error!("Error removing port forward command: {}", err);
                    }
                    break;
                }
                command if command.starts_with(HOST_FORWARD_LIST_COMMAND) => {
                    if let Err(err) = Self::port_forward_list(socket, chosen_adb_device).await {
                        error!("Error get port forward list: {}", err);
                    }
                    break;
                }
                command if command.starts_with(REVERSE_FORWARD_COMMAND) => {
                    if let Err(err) = Self::handle_port_reverse_command_set(socket, command.clone(), chosen_adb_device.clone()).await {
                        error!("Error handling port reverse command: {}", err);
                    }
                    break;
                }
                command if command.starts_with(REVERSE_KILL_FORWARD_COMMAND) => {
                    if let Err(err) = Self::port_reverse_remove(socket, command.clone(), chosen_adb_device.clone()).await {
                        error!("Error removing port reverse command: {}", err);
                    }
                    break;
                }
                command if command.starts_with(REVERSE_KILL_ALL_FORWARD_COMMAND) => {
                    if let Err(err) = Self::port_reverse_remove_all(socket, chosen_adb_device.clone()).await {
                        error!("Error removing all port reverse commands: {}", err);
                    }
                    break;
                }
                command if command.starts_with(REVERSE_FORWARD_LIST_COMMAND) => {
                    if let Err(err) = Self::port_reverse_list(socket, chosen_adb_device.clone()).await {
                        error!("Error getting port reverse list: {}", err);
                    }
                    break;
                }
                command if command.starts_with(SYNC_COMMAND) => {
                    match chosen_adb_device {
                        Some(ref adb_device) => {
                            Self::handle_sync_command(adb_device, socket).await;
                            break;
                        }
                        None => {
                            Self::handle_no_device_selected(socket).await;
                            break;
                        }
                    }
                }
                command if command.starts_with(SYNC_STAT_COMMAND_STR) || command.starts_with(SYNC_QUIT_COMMAND_STR)
                    || command.starts_with(SYNC_SEND_COMMAND_STR) || command.starts_with(SYNC_DATA_COMMAND_STR)
                    || command.starts_with(SYNC_RECV_COMMAND_STR) || command.starts_with(SYNC_DENT_COMMAND_STR) => {
                    Self::handle_invalid_sync_command(socket).await;
                    break;
                }
                _ => {
                    let _ = send_fail_with_response(socket, Some("Unknown ADBr server command".to_string()));
                    break;
                }
            }

            request = match Self::read_next_client_request(socket).await {
                Ok(req) => {
                    req
                }
                Err(err) => {
                    error!("Failed to read client request: {}", err);
                    return;
                }
            };
        }

        Self::close_client_connection(socket).await;
    }

    async fn on_find_device(serial_number: String, transport: Box<dyn Transport>, interface_type: InterfaceType) {
        ADB_SERVER_INSTANCE.adb_devices_hashmap.insert(serial_number.clone(), None);
        let device_type = match interface_type {
            InterfaceType::AndroidUsb(..) => AdbDeviceType::Usb,
            InterfaceType::AndroidTcp(..) => AdbDeviceType::Emulator,
        };
        let mut adb_device_connection = match AdbDeviceConnection::new(None, transport) {
            Ok(adb_device_connection) => adb_device_connection,
            Err(_) => {
                return;
            }
        };

        match adb_device_connection.connect(None).await {
            Ok(_) => {
                let adb_device = AdbDevice::new(serial_number.clone(), Arc::new(adb_device_connection), AdbDeviceStatus::Available, device_type, Duration::from_secs(Self::DEVICE_AVAILABLE_VERIFY_TIME_SECONDS));
                ADB_SERVER_INSTANCE.adb_devices_hashmap.insert(serial_number.clone(), Some(Arc::new(adb_device)));
                info!("Device {} connected successfully", serial_number);
                let serial_number_clone = serial_number.clone();
                tokio::spawn(async move {
                    match ActionConfig::load() {
                        Ok(action_config) => {
                            match execute_action(&action_config, &serial_number_clone, CONNECT_EVENT) {
                                Ok(()) => info!("Successfully executed connect actions for device {}", serial_number_clone),
                                Err(e) => error!("Failed to execute connect actions: {}", e),
                            }
                        }

                        Err(e) => error!("Failed to load actions configuration: {}", e),
                    }
                });
            }
            Err(AdbConnectionError::Unauthorized(err)) => {
                info!("Device {} is unauthorized: {}.", serial_number, err);
                let adb_device = AdbDevice::new(serial_number.clone(), Arc::new(adb_device_connection), AdbDeviceStatus::Unauthorized, device_type, Duration::from_secs(Self::DEVICE_UNAUTHORIZED_RECONNECT_TIME_SECONDS));
                ADB_SERVER_INSTANCE.adb_devices_hashmap.insert(serial_number.clone(), Some(Arc::new(adb_device)));
            }
            Err(AdbConnectionError::DeviceNotAvailable(err)) => {
                info!("Device {} is not available: {}.", serial_number, err);
                let adb_device = AdbDevice::new(serial_number.clone(), Arc::new(adb_device_connection), AdbDeviceStatus::Offline(err), device_type, Duration::from_secs(Self::DEVICE_NOT_AVAILABLE_RECONNECT_TIME_SECONDS));
                ADB_SERVER_INSTANCE.adb_devices_hashmap.insert(serial_number.clone(), Some(Arc::new(adb_device)));
            }
            Err(err) => {
                error!("Failed to connect to device {}: {}.", serial_number, err);
                adb_device_connection.close().await;
            }
        }
    }

    async fn monitor_connected_devices() {
        loop {
            if !ADB_SERVER_INSTANCE.adb_devices_hashmap.is_empty() {
                let serials_to_remove = stream::iter(ADB_SERVER_INSTANCE.adb_devices_hashmap.iter_mut())
                    .filter_map(|mut entry| async move {
                        let (serial_number, adb_device_option) = entry.pair_mut();
                        if let Some(adb_device) = adb_device_option.as_mut() {
                            if adb_device.is_monitoring_interval_passed() {
                                let adb_device_connection = adb_device.adb_device_connection();
                                let result = match adb_device.adb_device_status() {
                                    AdbDeviceStatus::Available => {
                                        match adb_device_connection.verify_connection_status().await {
                                            Ok(_) => None,
                                            Err(_) => {
                                                info!("Device {} disconnected", serial_number);
                                                adb_device.close_device_gracefully().await;
                                                adb_device_connection.close().await;
                                                let serial_number_clone = serial_number.clone();
                                                tokio::spawn(async move {
                                                    match ActionConfig::load() {
                                                        Ok(action_config) => {
                                                            match execute_action(&action_config, &serial_number_clone, DISCONNECT_EVENT) {
                                                                Ok(()) => info!("Successfully executed disconnect actions for device {}", serial_number_clone),
                                                                Err(e) => error!("Failed to execute disconnect actions: {}", e),
                                                            }
                                                        }
                                                        Err(e) => error!("Failed to load actions configuration: {}", e),
                                                    }
                                                });
                                                Some(serial_number.clone())
                                            }
                                        }
                                    }
                                    AdbDeviceStatus::Unauthorized | AdbDeviceStatus::Offline(_) => {
                                        info!("Attempting to reconnect to device {}", serial_number);
                                        Some(serial_number.clone())
                                    }
                                };
                                adb_device.update_last_monitored();
                                result
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<String>>()
                    .await;

                for serial_number in serials_to_remove {
                    ADB_SERVER_INSTANCE.adb_devices_hashmap.remove(&serial_number);
                }
            }
            time::sleep(Duration::from_millis(Self::MONITORING_INTERVAL_MS)).await;
        }
    }

    fn is_new_device(serial_number: String) -> bool {
        !ADB_SERVER_INSTANCE.adb_devices_hashmap.contains_key(&serial_number)
    }

    async fn send_version_response(socket: &mut TcpStream) {
        let response = format!("{}{:08x}", OKAY, ADB_SERVER_VERSION);
        match send_full_response(socket, response).await {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to send version response: {}", err);
            }
        };
    }

    async fn read_next_client_request(socket: &mut TcpStream) -> Result<String, AdbServerError> {
        match read_request_from_socket(socket).await {
            Ok(request) => Ok(request),
            Err(AdbIoError::ConnectionClosed(_)) | Err(AdbIoError::TimeoutError) => {
                Err(AdbServerError::RequestError("Connection closed or timed out while reading request".to_string()))
            }
            Err(_) => {
                let _ = send_full_response(socket, String::from("Request format is incorrect")).await;
                Err(AdbServerError::RequestError("Invalid request format".to_string()))
            }
        }
    }

    async fn close_client_connection(socket: &mut TcpStream)  {
        if let Err(err) = socket.shutdown().await {
            warn!("Failed to shutdown socket: {}", err);
        }
    }
}