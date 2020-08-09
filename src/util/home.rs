use std::env;

#[cfg(os = "windows")]
pub fn home() -> Option<String> {
    match env::var("UserProfile") {
        Ok(pathstr) => Some(pathstr),
        _ => None,
    }
}

#[cfg(not(os = "windows"))]
pub fn home() -> Option<String> {
    match env::var("HOME") {
        Ok(pathstr) => Some(pathstr),
        _ => None,
    }
}
