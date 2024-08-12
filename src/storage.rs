use anyhow::anyhow;
use github_backend::GithubBackend;
use serde_json::Value;

mod github_backend;

pub struct Storage {
    github: GithubBackend,
}

impl Storage {
    pub fn new(owner: &str, repo: &str, branch: &str, token: Option<String>) -> Self {
        Self {
            github: GithubBackend::new(owner, repo, branch, token),
        }
    }

    pub async fn save_crate(
        &self,
        name: &str,
        vers: &str,
        meta: Vec<u8>,
        pkg: Vec<u8>,
    ) -> anyhow::Result<()> {
        if name.len() < 4 {
            return Err(anyhow!("Name too short"));
        }

        let be = &name[0..2];
        let md = &name[2..4];

        let meta_path = format!("{be}/{md}/{name}");
        let crate_path = format!("{name}/{vers}/download");

        log::info!("prepare for create new pkg {meta_path}, {crate_path}");

        if self.github.file_exits(&crate_path).await.is_ok() {
            log::error!("already existed");
            return Err(anyhow!("Already existed"));
        }

        let mut json = serde_json::from_slice::<Value>(&meta).expect("Should convert meta to json");
        if json.get("cksum").is_none() {
            json["cksum"] = sha256::digest(&pkg).into();
        }
        if let Some(deps) = json.get_mut("deps") {
            if let Some(deps) = deps.as_array_mut() {
                for dep in deps {
                    dep["req"] = dep["version_req"].clone();
                }
            }
        }
        let meta_new_buf = json.to_string().as_bytes().to_vec();
        self.github.append(&meta_path, meta_new_buf, true).await?;
        self.github.create(&crate_path, pkg).await?;
        Ok(())
    }

    pub async fn get_crate(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        if name.len() < 4 {
            return Err(anyhow!("Name too short"));
        }
        let be = &name[0..2];
        let md = &name[2..4];

        self.github.get_binary(&format!("{be}/{md}/{name}")).await
    }

    pub async fn down_crate(&self, name: &str, vers: &str) -> anyhow::Result<Vec<u8>> {
        if name.len() < 4 {
            return Err(anyhow!("Name too short"));
        }

        let crate_path = format!("{name}/{vers}/download");
        self.github.get_binary(&crate_path).await
    }
}
