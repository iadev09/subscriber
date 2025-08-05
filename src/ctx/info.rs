use super::error::Error;

#[allow(unused)]
pub struct Info {
    pub app: String,
    pub user: String,
    pub hostname: String,
    pub work_dir: String
}

#[allow(unused)]
impl Info {
    pub fn get_current_user(&self) -> &str {
        self.user.as_ref()
    }

    pub(crate) fn get_working_dir(&self) -> &str {
        self.user.as_ref()
    }

    pub fn get_hostname(&self) -> &str {
        self.hostname.as_ref()
    }

    pub(crate) fn my_name(&self) -> &str {
        self.app.as_ref()
    }
}

impl Info {
    pub fn new(
        user: String,
        hostname: String,
        work_dir: String
    ) -> Self {
        let app = env!("CARGO_PKG_NAME").to_string();
        Self { app, user, hostname, work_dir }
    }

    pub fn from_env() -> Result<Self, Error> {
        let user = std::env::var("USER")?;
        let work_dir = std::env::current_dir()?.to_string_lossy().to_string();
        let hostname =
            hostname::get()?.to_string_lossy().split('.').next().unwrap_or_default().to_string();
        Ok(Self::new(user, hostname, work_dir))
    }
}
