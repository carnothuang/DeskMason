use crate::widget::WidgetKind;

/// 瓦片类型的静态声明：宿主只认这张表，不再到处写 `match kind`。
///
/// 新增一种类型若需要改动多处 kind 分支，漏一处就会静默失效。
/// 这里把"类型知识"集中成一条记录，新增类型只改 `WidgetKind` 与这张表，
/// 漏写会被 `every_kind_has_a_descriptor` 测试挡住。
#[derive(Debug, Clone, Copy)]
pub struct WidgetDescriptor {
    pub kind: WidgetKind,
    /// 同时存在多个实例（文件、随记、待办、天气、音乐、时光），搜索只允许一个。
    pub multi_instance: bool,
    /// 设置页里的分区 id；`None` 表示没有独立设置区。
    pub settings_section: Option<&'static str>,
    /// 默认尺寸（逻辑像素），仅用于新建时的初值。
    pub default_size: (u32, u32),
    /// 最小尺寸。
    pub min_size: (u32, u32),
    /// 能否折叠成胶囊。
    pub capsule_capable: bool,
}

pub struct WidgetRegistry {
    descriptors: Vec<WidgetDescriptor>,
}

impl WidgetRegistry {
    pub fn builtin() -> Self {
        use WidgetKind::*;
        Self {
            descriptors: vec![
                WidgetDescriptor {
                    kind: File,
                    multi_instance: true,
                    settings_section: Some("file"),
                    default_size: (360, 480),
                    min_size: (50, 50),
                    capsule_capable: true,
                },
                WidgetDescriptor {
                    kind: QuickCapture,
                    multi_instance: true,
                    settings_section: Some("quickCapture"),
                    default_size: (360, 480),
                    min_size: (200, 160),
                    capsule_capable: true,
                },
                WidgetDescriptor {
                    kind: Todo,
                    multi_instance: true,
                    settings_section: Some("todo"),
                    default_size: (420, 520),
                    min_size: (240, 200),
                    capsule_capable: true,
                },
                WidgetDescriptor {
                    kind: Weather,
                    multi_instance: true,
                    settings_section: Some("weather"),
                    default_size: (360, 420),
                    min_size: (200, 160),
                    capsule_capable: true,
                },
                WidgetDescriptor {
                    kind: Music,
                    multi_instance: true,
                    settings_section: Some("music"),
                    default_size: (360, 260),
                    min_size: (200, 120),
                    capsule_capable: true,
                },
                WidgetDescriptor {
                    kind: Glance,
                    multi_instance: true,
                    settings_section: Some("glance"),
                    default_size: (420, 260),
                    min_size: (120, 90),
                    capsule_capable: true,
                },
            ],
        }
    }

    pub fn all(&self) -> &[WidgetDescriptor] {
        &self.descriptors
    }

    pub fn get(&self, kind: WidgetKind) -> Option<&WidgetDescriptor> {
        self.descriptors.iter().find(|d| d.kind == kind)
    }

    pub fn describe(&self, kind: WidgetKind) -> &WidgetDescriptor {
        self.get(kind)
            .unwrap_or_else(|| panic!("missing descriptor for {kind:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_descriptor() {
        let registry = WidgetRegistry::builtin();
        for kind in WidgetKind::ALL {
            assert!(
                registry.get(kind).is_some(),
                "{kind:?} 没有 WidgetDescriptor；新增类型必须同时登记"
            );
        }
        assert_eq!(registry.all().len(), WidgetKind::ALL.len());
    }

    #[test]
    fn descriptors_are_unique() {
        let registry = WidgetRegistry::builtin();
        let mut kinds: Vec<_> = registry.all().iter().map(|d| d.kind).collect();
        kinds.sort_unstable();
        let before = kinds.len();
        kinds.dedup();
        assert_eq!(before, kinds.len());
    }

    #[test]
    fn default_size_respects_minimum() {
        let registry = WidgetRegistry::builtin();
        for d in registry.all() {
            assert!(d.default_size.0 >= d.min_size.0);
            assert!(d.default_size.1 >= d.min_size.1);
        }
    }

    #[test]
    fn file_tile_is_multi_instance() {
        let registry = WidgetRegistry::builtin();
        assert!(registry.describe(WidgetKind::File).multi_instance);
    }
}
