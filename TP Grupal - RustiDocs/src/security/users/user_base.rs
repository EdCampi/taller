use super::super::types::{Password, ValidationError};
use super::permissions::Permissions;
use super::user::User;
use std::collections::HashMap;

pub struct UserBase {
    users: HashMap<String, User>,
}

impl UserBase {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    pub fn validate_user(
        &self,
        username: &str,
        password: &Password,
    ) -> Result<Permissions, ValidationError> {
        match self.users.get(username) {
            None => Err(ValidationError::UserNotFound),
            Some(user) => {
                if user.is_password(password) {
                    Ok(user.get_permission())
                } else {
                    Err(ValidationError::IncorrectPassword)
                }
            }
        }
    }

    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.username.to_string(), user);
    }

    pub fn user_exist(&self, username: &str) -> bool {
        self.users.contains_key(username)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_add_user() {
        let username = "JoseMaria";
        let user = User::new(username.to_string(), "1234".to_string(), Permissions::new());

        let mut user_base = UserBase::new();
        user_base.add_user(user);

        assert_eq!(user_base.user_exist(username), true);
    }

    #[test]
    fn test_succesfully_validate_user() {
        let user_permission = Permissions::new();
        let username = "JoseMaria";
        let password = "1234".to_string();
        let user = User::new(
            username.to_string(),
            password.to_string(),
            user_permission.clone(),
        );

        let mut user_base = UserBase::new();
        user_base.add_user(user);

        let validation_result = user_base.validate_user(username, &password).unwrap();

        assert_eq!(user_permission, validation_result);
    }

    #[test]
    fn test_validate_user_dont_exist() {
        let user_base = UserBase::new();
        let username = "JoseMaria";
        let password = "1234".to_string();
        let expected_error = ValidationError::UserNotFound;

        let validation_result = user_base.validate_user(username, &password);

        assert_eq!(Err(expected_error), validation_result);
    }

    #[test]
    fn test_validate_incorrect_password() {
        let username = "JoseMaria";
        let user_password = "1234".to_string();
        let invalid_password = "54321".to_string();
        let user = User::new(username.to_string(), user_password, Permissions::new());
        let expected_error = ValidationError::IncorrectPassword;
        let mut user_base = UserBase::new();
        user_base.add_user(user);

        let validation_result = user_base.validate_user(username, &invalid_password);

        assert_eq!(Err(expected_error), validation_result);
    }
}
