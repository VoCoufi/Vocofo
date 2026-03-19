#[allow(dead_code)]
pub enum MessageEnum {
    AppTitle,
    ErrorNotFound,
    ErrorInvalidInput(String), // Can include dynamic data
}

impl MessageEnum {
    pub fn as_str(&self) -> String {
        match self {
            MessageEnum::AppTitle => "Vocofo File Manager".to_string(),
            MessageEnum::ErrorNotFound => "Item not found".to_string(),
            MessageEnum::ErrorInvalidInput(details) => {
                format!("Invalid input: {}", details)
            }
        }
    }
}
