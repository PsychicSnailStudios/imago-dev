use std::sync::Mutex;

pub struct AppState {
    pub active_profile_uid: Mutex<String>,
    pub spotify_verifier: Mutex<String>,
}

impl AppState {
    pub fn new(uid: String) -> Self {
        AppState {
            active_profile_uid: Mutex::new(uid),
            spotify_verifier: Mutex::new(String::new()),
        }
    }

    pub fn get_uid(&self) -> String {
        self.active_profile_uid.lock().unwrap().clone()
    }

    pub fn set_uid(&self, uid: String) {
        *self.active_profile_uid.lock().unwrap() = uid;
    }

    pub fn set_spotify_verifier(&self, verifier: String) {
        let mut v = self.spotify_verifier.lock().unwrap();
        *v = verifier;
    }

    pub fn take_spotify_verifier(&self) -> String {
        let mut v = self.spotify_verifier.lock().unwrap();
        std::mem::take(&mut *v)
    }
}
