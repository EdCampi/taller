use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::security::users::permissions::Permissions;
use crate::security::users::user::User;
use crate::security::{
    users::user_base::UserBase, // suponiendo que está en userbase.rs
};

pub fn load_users_from_acl(path: &str) -> std::io::Result<UserBase> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut user_base = UserBase::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();

        // Check "user" keyword
        if parts.next() != Some("user") {
            continue;
        }

        // Username
        let username = match parts.next() {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Password
        let password_token = match parts.next() {
            Some(p) if p.starts_with('>') => p.trim_start_matches('>').to_string(),
            _ => continue,
        };

        let mut permissions = Permissions::new();
        if parts.clone().last().unwrap_or(&"") == "*" {
            permissions.set_super();
        } else {
            for token in parts {
                if let Some(instr) = token.strip_prefix('+') {
                    permissions.add_instruction(instr.to_string());
                }
            }
        }

        let user = User::new(username, password_token, permissions);
        user_base.add_user(user);
    }

    Ok(user_base)
}
