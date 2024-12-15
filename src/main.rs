use std::env::args;
use std::process::{Command, exit, Stdio};
use crate::adb::server::server::AdbServer;
use crate::constants::{DEFAULT_ADB_SERVER_PORT, EXIT_FAILURE, LOCAL_IP};
use crate::utils::utils::is_port_available;

mod transport;
mod utils;
mod constants;
mod scanners;
mod adb;
mod logging;

const EXTERNAL_ADB_SERVER_IP: &str = "0.0.0.0";
const LISTEN_ON_ALL_INTERFACES_FLAG: &str = "-a";
const PORT_FLAG: &str = "-p";
const START_COMMAND: &str = "start-server";
const KILL_COMMAND: &str = "kill-server";
const RESTART_COMMAND: &str = "restart-server";
const BACKGROUND_SERVER_COMMAND: &str = "background-server";
const PGREP_PATTERN: &str = "adbr-server.*background-server";
const PGREP_COMMAND: &str = "pgrep";
const PKILL_COMMAND: &str = "pkill";
const FLAG_F: &str = "-f";
const PORT_IN_USE_ERROR: i32 = 98;

fn print_usage() {
    println!("Usage: adbr-server <command> [options]");
    println!("\nCommands:");
    println!("  start-server    Start the ADBR server in background");
    println!("  kill-server     Kill the running ADBR server");
    println!("  restart-server  Restart the ADBR server");
    println!("\nOptions:");
    println!("  -a             Listen on all network interfaces (default: localhost only)");
    println!("  -p <port>      Specify port number (default: 5037)");
    println!("\nExamples:");
    println!("  adbr-server start-server");
    println!("  adbr-server start-server -a -p 5038");
    println!("  adbr-server kill-server");
    println!("  adbr-server restart-server");
}

fn is_server_running() -> bool {
    if let Ok(output) = Command::new(PGREP_COMMAND)
        .args([FLAG_F, PGREP_PATTERN])
        .output()
    {
        !output.stdout.is_empty()
    } else {
        false
    }
}

async fn kill_server() -> bool {
    if let Ok(output) = Command::new(PKILL_COMMAND)
        .args([FLAG_F, PGREP_PATTERN])
        .output()
    {
        if output.status.success() {
            println!("ADBR server killed successfully");
            true
        } else {
            println!("No ADBR server running");
            false
        }
    } else {
        eprintln!("Failed to kill ADBR server");
        false
    }
}

async fn start_background_server(server_listen_address: String, port: Option<u16>) {
    if is_server_running() {
        println!("ADBR server is already running");
        return;
    }

    let port_number = port.unwrap_or(DEFAULT_ADB_SERVER_PORT);

    if !is_port_available(&server_listen_address, port_number).await {
        eprintln!("ADBR-Server Error: Port {} is already in use", port_number);
        exit(PORT_IN_USE_ERROR);
    }

    let current_exe = std::env::current_exe().expect("Failed to get current executable path");

    let mut command = Command::new(current_exe);
    command.arg(BACKGROUND_SERVER_COMMAND)
        .arg(server_listen_address.clone())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(p) = port {
        command.arg(p.to_string());
    }

    match command.spawn() {
        Ok(_) => {
            println!("ADBR server started successfully");
            println!("Listening on: {}:{}", server_listen_address, port_number);
        }
        Err(err) => {
            eprintln!("Failed to start background server: {}", err);
            exit(EXIT_FAILURE);
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = args().collect();

    if args.len() <= 1 {
        print_usage();
        exit(EXIT_FAILURE);
    }

    match args[1].as_str() {
        START_COMMAND => {
            let mut server_listen_address = String::from(LOCAL_IP);

            if args.contains(&String::from(LISTEN_ON_ALL_INTERFACES_FLAG)) {
                server_listen_address = String::from(EXTERNAL_ADB_SERVER_IP);
            }

            let port = args.iter().position(|arg| arg == PORT_FLAG)
                .and_then(|i| args.get(i + 1))
                .and_then(|port_str| port_str.parse::<u16>().ok());

            start_background_server(server_listen_address, port).await;
        }
        RESTART_COMMAND => {
            kill_server().await;
            let mut server_listen_address = String::from(LOCAL_IP);

            if args.contains(&String::from(LISTEN_ON_ALL_INTERFACES_FLAG)) {
                server_listen_address = String::from(EXTERNAL_ADB_SERVER_IP);
            }

            let port = args.iter().position(|arg| arg == PORT_FLAG)
                .and_then(|i| args.get(i + 1))
                .and_then(|port_str| port_str.parse::<u16>().ok());

            start_background_server(server_listen_address, port).await;
        }
        BACKGROUND_SERVER_COMMAND => {
            init_logs();
            let address = if args.len() > 2 { args[2].clone() } else { String::from(LOCAL_IP) };
            let port = args.get(3).and_then(|p| p.parse::<u16>().ok());

            let adb_server = AdbServer::get_instance();
            adb_server.init(address, port).await;
            tokio::signal::ctrl_c().await.expect("failed to listen for event");
        }
        KILL_COMMAND => {
            kill_server().await;
        }
        _ => {
            println!("Unknown command: {}", args[1]);
            print_usage();
            exit(EXIT_FAILURE);
        }
    }

    fn init_logs() {
        logging::init()
            .map_err(|e| {
                eprintln!("Failed to initialize logging: {}", e);
                e
            })
            .expect("Failed to setup logging");

        tracing::info!("ADBR Server starting up...");
    }
}