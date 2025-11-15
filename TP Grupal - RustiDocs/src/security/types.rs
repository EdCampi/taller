pub type Password = String; // => ESTO ES PARA QUE PUEDA SER CAMBIADO POR UNA CONTRASEÑA CIFRADA SI HACE FALTA

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    UserNotFound,
    IncorrectPassword,
}
