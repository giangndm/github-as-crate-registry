use anyhow::anyhow;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use octocrab::params::repos::Reference;

pub struct GithubBackend {
    owner: String,
    repo: String,
    branch: String,
    instance: octocrab::Octocrab,
}

impl GithubBackend {
    pub fn new(owner: &str, repo: &str, branch: &str, token: Option<String>) -> Self {
        let instance = if let Some(token) = token {
            octocrab::OctocrabBuilder::new()
                .personal_token(token)
                .build()
                .expect("Build instance")
        } else {
            octocrab::OctocrabBuilder::new()
                .build()
                .expect("Build instance")
        };
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: branch.to_string(),
            instance,
        }
    }

    pub async fn file_exits(&self, path: &str) -> anyhow::Result<()> {
        self.get_binary(path).await?;
        Ok(())
    }

    pub async fn get_sha(&self, path: &str) -> anyhow::Result<String> {
        let res = self
            .instance
            .repos(&self.owner, &self.repo)
            .get_content()
            .path(path)
            .r#ref(self.branch.clone())
            .send()
            .await?;
        res.items
            .last()
            .ok_or(anyhow!("NotFound"))
            .map(|i| i.sha.clone())
    }

    pub async fn get_binary(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let res = self
            .instance
            .repos(&self.owner, &self.repo)
            .raw_file(Reference::Branch(self.branch.to_string()), path)
            .await?;

        if res.status() != 200 {
            return Err(anyhow!("Not found"));
        }

        let mut buf = Vec::new();
        let mut body = res.into_body().into_data_stream();
        while let Some(Ok(chunk)) = body.next().await {
            log::info!("{path}: chunk {}", chunk.len());
            buf.append(&mut chunk.to_vec());
        }

        Ok(buf)
    }

    pub async fn append(
        &self,
        path: &str,
        mut content: Vec<u8>,
        new_line: bool,
    ) -> anyhow::Result<()> {
        if let Ok(mut old_data) = self.get_binary(path).await {
            let sha = self.get_sha(&path).await?;
            if new_line {
                old_data.push('\n' as u8);
            }
            old_data.append(&mut content);
            self.instance
                .repos(&self.owner, &self.repo)
                .update_file(path, "Append content to file", &old_data, sha)
                .send()
                .await?;
        } else {
            self.instance
                .repos(&self.owner, &self.repo)
                .create_file(path, "Add new file", &content)
                .send()
                .await?;
        }
        Ok(())
    }

    pub async fn create(&self, path: &str, content: Vec<u8>) -> anyhow::Result<()> {
        self.instance
            .repos(&self.owner, &self.repo)
            .create_file(path, "Add new file", &content)
            .send()
            .await?;
        Ok(())
    }
}
