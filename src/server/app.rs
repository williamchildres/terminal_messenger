use crate::HashMap;
pub struct App {
    connected_users: HashMap<String, String>,
}

impl App {
    pub fn new() -> App {
        // Initialize and return a new instance of `App`
        App {
            connected_users: HashMap::new(),
        }
    }

    pub async fn add_connected_user(&mut self, user_id: String, username: String) {
        self.connected_users.insert(user_id, username);
    }

    pub async fn get_connected_user(&self, user_id: &str) -> Option<&String> {
        self.connected_users.get(user_id)
    }

    pub async fn remove_connected_user(&mut self, user_id: &str) -> Option<String> {
        self.connected_users.remove(user_id)
    }
}
