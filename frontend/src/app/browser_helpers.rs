use web_sys::window;

use super::log;

pub fn get_value_from_storage(key: impl AsRef<str>) -> Option<String> {
    let storage = window().unwrap().local_storage();
    let storage = match storage {
        Ok(s) => s,
        Err(e) => {
            log(&format!(
                "Failed to get storage. Error: {}",
                e.as_string()
                    .unwrap_or("<Could not extract error as string>".to_string())
            ));
            return None;
        }
    };

    let storage = match storage {
        Some(s) => s,
        None => {
            log(&format!("Failed to get storage. Got None.",));
            return None;
        }
    };

    match storage.get_item(key.as_ref()) {
        Ok(res) => res,
        Err(e) => {
            log(&format!(
                "Failed to get storage. Error: {}",
                e.as_string()
                    .unwrap_or("<Could not extract error as string>".to_string())
            ));
            return None;
        }
    }
}

pub fn set_value_in_storage(key: impl AsRef<str>, value: impl AsRef<str>) {
    let storage = window().unwrap().local_storage();
    let storage = match storage {
        Ok(s) => s,
        Err(e) => {
            log(&format!(
                "Failed to get storage. Error: {}",
                e.as_string()
                    .unwrap_or("<Could not extract error as string>".to_string())
            ));
            return;
        }
    };

    let storage = match storage {
        Some(s) => s,
        None => {
            log(&format!("Failed to get storage. Got None.",));
            return;
        }
    };

    match storage.set_item(key.as_ref(), value.as_ref()) {
        Ok(_) => (),
        Err(e) => {
            log(&format!(
                "Failed to get storage. Error: {}",
                e.as_string()
                    .unwrap_or("<Could not extract error as string>".to_string())
            ));
            return;
        }
    }
}
