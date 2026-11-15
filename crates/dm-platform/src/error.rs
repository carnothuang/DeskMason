#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("无法连接显示服务器: {0}")]
    Connect(#[from] x11rb::errors::ConnectError),
    #[error("X11 请求失败: {0}")]
    Connection(#[from] x11rb::errors::ConnectionError),
    #[error("X11 应答失败: {0}")]
    Reply(#[from] x11rb::errors::ReplyError),
    #[error("X11 请求或资源 ID 分配失败: {0}")]
    X11(#[from] x11rb::errors::ReplyOrIdError),
    #[error("当前会话不是 X11（DISPLAY 缺失）")]
    NoX11Display,
    #[error("该热键组合已被其他程序占用: {0}")]
    HotkeyConflict(String),
    #[error("文件系统操作失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("路径无效: {0}")]
    InvalidPath(String),
    #[error("无法启动外部程序: {0}")]
    Spawn(String),
    #[error("目录监听失败: {0}")]
    Watch(#[from] notify::Error),
    #[error("网络请求失败: {0}")]
    Http(String),
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
}

pub type PlatformResult<T> = Result<T, PlatformError>;
