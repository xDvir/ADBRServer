#[derive(Debug)]
#[allow(dead_code)]
pub enum SyncCommand {
    Send { mode: u32, size: u32, path: String },
    Data { size: u32 },
    Done { mtime: u32 },
    Stat { path: String },
    Recv { path: String },
    List { path: String },
    Dent { mode: u32, size: u32, mtime: u32, name: String },
    Quit,
}
