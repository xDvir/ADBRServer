pub const LOCAL_IP: &str = "127.0.0.1";

pub const OKAY: &str = "OKAY";
pub const FAIL: &str = "FAIL";
pub const B_FAIL: &[u8] = b"FAIL";

pub const ADB_SERVER_VERSION: u32 = 0x01000000;

pub const MAX_ADB_DATA: u32 = 1024 * 1024;

pub const DEFAULT_ADB_SERVER_PORT: u16 = 5037;

pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024; // 64K
pub const ADB_MESSAGE_SIZE: usize = 24;

pub const BINARY_SHELL_COMMAND: &[u8] = b"shell:";
pub const BINARY_SHELL_COMMAND_NULL: &[u8] = b"shell:\0";
pub const SHELL_COMMAND: &str = "shell:";

pub const ADB_PRIVATE_KEY_FILE: &'static str = "adbkey";
pub const ADB_PUBLIC_KEY_FILE: &'static str = "adbkey.pub";
pub const HOST: &str = "host::";

pub const ZERO: u32 = 0;
pub const ONE: u32 = 1;
pub const NULL_TERMINATOR: char = '\0';
pub const EXIT_FAILURE: i32 = 1;

pub const HOST_TRANSPORT_ANY_COMMAND: &str = "host:transport-any";
pub const HOST_EMULATOR_ANY_COMMAND: &str = "host:transport-local";
pub const HOST_USB_ANY_COMMAND: &str = "host:transport-usb";
pub const HOST_TRANSPORT_COMMAND: &str = "host:transport:";
pub const HOST_FORWARD_COMMAND: &str = "host:forward:";
pub const HOST_KILL_FORWARD_COMMAND: &str = "host:killforward:";
pub const HOST_FORWARD_KILL_ALL_COMMAND: &str = "host:killforward-all";
pub const HOST_FORWARD_LIST_COMMAND: &str = "host:list-forward";
pub const REVERSE_FORWARD_COMMAND: &str = "reverse:forward:";
pub const REVERSE_KILL_FORWARD_COMMAND: &str = "reverse:killforward:";
pub const REVERSE_KILL_ALL_FORWARD_COMMAND: &str = "reverse:killforward-all";
pub const REVERSE_FORWARD_LIST_COMMAND: &str = "reverse:list-forward";
pub const HOST_SERIALNO_COMMAND: &str = "host:get-serialno";
pub const HOST_GET_DEVPATH_COMMAND: &str = "host:get-devpath";
pub const HOST_GET_STATE_COMMAND: &str = "host:get-state";

pub const SYNC_COMMAND: &str = "sync:";

pub const SYNC_SEND_COMMAND: &[u8] = b"SEND";
pub const SYNC_SEND_COMMAND_STR: &str = "SEND";

pub const SYNC_DATA_COMMAND: &[u8] = b"DATA";
pub const SYNC_DATA_COMMAND_STR: &str = "DATA";

pub const SYNC_DONE_COMMAND: &[u8] = b"DONE";
pub const SYNC_DONE_COMMAND_STR: &str = "DONE";

pub const SYNC_QUIT_COMMAND: &[u8] = b"QUIT";
pub const SYNC_QUIT_COMMAND_STR: &str = "QUIT";

pub const SYNC_STAT_COMMAND: &[u8] = b"STAT";
pub const SYNC_STAT_COMMAND_STR: &str = "STAT";

pub const SYNC_RECV_COMMAND: &[u8] = b"RECV";
pub const SYNC_RECV_COMMAND_STR: &str = "RECV";

pub const SYNC_LIST_COMMAND: &[u8] = b"LIST";
pub const SYNC_LIST_COMMAND_STR: &str = "LIST";

pub const SYNC_DENT_COMMAND: &[u8] = b"DENT";
pub const SYNC_DENT_COMMAND_STR: &str = "DENT";
pub const DENT_HEADER_SIZE: usize = 16;
pub const DENT_NAME_LENGTH_SIZE: usize = 4;
pub const DENT_MIN_SIZE: usize = DENT_HEADER_SIZE + DENT_NAME_LENGTH_SIZE;

pub const HOST_VERSION_COMMAND: &str = "host:version";
pub const HOST_DEVICES_COMMAND: &str = "host:devices";
pub const REBOOT_COMMAND: &str = "reboot:";
pub const REMOUNT_COMMAND: &str = "remount:";
pub const ROOT_COMMAND: &str = "root:";
pub const UNROOT_COMMAND: &str = "unroot:";

pub const DISABLE_VERITY_COMMAND: &str = "disable-verity:";
pub const ENABLE_VERITY_COMMAND: &str = "enable-verity:";

pub const SHELL_BUGREPORT_COMMAND: &str = "shell:bugreportz";
pub const SYNC_RECV_GET_DATA_TIME_SECONDS: f64 = 1.0;


pub const NO_REBIND_PORT_PREFIX: &str = "norebind:";

pub const CNXN_CODE: u32 = 0x4E584E43;
pub const AUTH_CODE: u32 = 0x48545541;
pub const CLSE_CODE: u32 = 0x45534C43;
pub const OKAY_CODE: u32 = 0x59414B4F;
pub const OPEN_CODE: u32 = 0x4E45504F;
pub const WRTE_CODE: u32 = 0x45545257;

pub const ABSTRACT_SOCKET_PREFIX: &str = "\0";
pub const RESERVED_SOCKET_PREFIX: &str = "/reserved/";
pub const DEV_SOCKET_PREFIX: &str = "/dev/";

pub const CONNECT_EVENT: &str = "connect";
pub const DISCONNECT_EVENT: &str = "disconnect";