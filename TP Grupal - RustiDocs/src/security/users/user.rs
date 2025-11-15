use crate::security::types::Password;
use crate::security::users::permissions::Permissions;

pub struct User {
    pub username: String,
    password: Password,
    pub allowed_instructios: Permissions,
}

impl User {
    pub fn new(username: String, password: Password, allowed_instructios: Permissions) -> Self {
        User {
            username,
            password,
            allowed_instructios,
        }
    }

    pub fn is_password(&self, password: &Password) -> bool {
        password == &self.password
    }

    pub fn get_permission(&self) -> Permissions {
        return self.allowed_instructios.clone();
    }
}
