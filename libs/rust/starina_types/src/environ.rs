pub struct Environ {
    pub data: &'static [u8],
}

impl Environ {
    pub unsafe fn from_raw(data: &'static [u8]) -> Self {
        Self { data }
    }

    pub fn raw(&self) -> &'static [u8] {
        self.data
    }

    /// Parses this environment into the given type.
    ///
    /// This method takes `self` because it may contain handles, that are
    /// not `Copy`.
    pub fn parse<E>(self) -> Result<E, serde_json::Error>
    where
        E: serde::de::DeserializeOwned,
    {
        serde_json::from_slice(self.data)
    }
}
