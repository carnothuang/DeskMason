//! DeskMason 领域层。
//!
//! 这里只有平台无关的规则：瓦片是什么、布局怎么存、设置怎么迁移。
//! 不出现窗口、D-Bus、文件系统之外的系统调用。

pub mod file_tile;
pub mod file_tile_settings;
pub mod geometry;
pub mod glance;
pub mod layout;
pub mod quick_capture;
pub mod registry;
pub mod todo;
pub mod weather;
pub mod widget;

pub use file_tile::{FileEntry, FileSortMode};
pub use geometry::{MonitorId, Rect};
pub use layout::{LayoutDocument, Placement};
pub use registry::{WidgetDescriptor, WidgetRegistry};
pub use widget::{WidgetConfig, WidgetKind};
