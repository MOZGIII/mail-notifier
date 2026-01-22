//! Slotmap key to use.

slotmap::new_key_type! {
    pub struct Key;
}

impl From<Key> for tray_icon::menu::MenuId {
    fn from(value: Key) -> Self {
        let value = value.0.as_ffi();
        tray_icon::menu::MenuId(value.to_string())
    }
}

impl TryFrom<tray_icon::menu::MenuId> for Key {
    type Error = <u64 as std::str::FromStr>::Err;

    fn try_from(value: tray_icon::menu::MenuId) -> Result<Self, Self::Error> {
        let value = value.0.parse()?;
        let value = slotmap::KeyData::from_ffi(value);
        Ok(value.into())
    }
}
