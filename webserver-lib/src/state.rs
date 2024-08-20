pub struct ServerState {
    /// Permissive is true when all functionalities are allowed in the API server
    /// when false, only limited functionality exists.
    is_permissive: bool,
}

impl ServerState {
    pub fn new(is_permissive: bool) -> Self {
        Self { is_permissive }
    }

    pub fn is_permissive(&self) -> bool {
        self.is_permissive
    }
}
