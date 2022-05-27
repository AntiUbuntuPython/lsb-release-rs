pub(crate) trait LSBInfo {
    fn id(&self) -> Option<String>;

    fn description(&self) -> Option<String>;

    fn release(&self) -> Option<String>;

    fn codename(&self) -> Option<String>;

    fn lsb_version(&self) -> Option<Vec<String>>;
}

pub(crate) fn grub_info() -> impl LSBInfo {
    todo!("not yet")
}