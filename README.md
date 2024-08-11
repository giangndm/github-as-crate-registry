# Private-Crate-Hub

Private-Crate-Hub is a private Rust crate registry designed to store all your crate data inside a GitHub repository. It provides an easy and secure way to manage and distribute your private Rust crates, ensuring that your code is versioned, backed up, and accessible only to authorized users.

## Why Do I Need It?

When working with private Rust projects, especially when using tools like `release-plz`, publishing to the public [crates.io](https://crates.io) registry isn't an option. Managing a private registry typically involves setting up a SQL database, S3 storage, and other infrastructure that can be overkill for small projects.

For my use case, I only have a small number of packages (~100), so I wanted a solution that was simpler and didn't require extensive infrastructure. GitHub, being a robust platform with built-in version control, security, and backup features, is an ideal choice for this purpose. By using GitHub as the storage backend, I eliminate the need to maintain a database, handle backups, or incur additional costs. Private-Crate-Hub makes managing private crates straightforward, leveraging GitHub's capabilities to keep things simple and efficient.

## Features

- **Private Registry**: Keep your Rust crates private and secure, accessible only to your team or organization.
- **GitHub Integration**: Store all crate data inside a specified GitHub repository, leveraging GitHub's robust version control and security features.
- **Seamless Cargo Integration**: Works seamlessly with Cargo, the Rust package manager, making it easy to use in your existing Rust projects.

## Getting Started

#### Using Docker

You can quickly get started with Private-Crate-Hub using Docker. This method requires minimal setup and ensures that the environment is consistent.

1. **Pull the Docker Image**:

```bash
docker pull giangndm/private-crate-hub:latest
```

2. ** Run with Environment Variables **

You can run the Private-Crate-Hub Docker container with the following environment variables:

- GITHUB_TOKEN: Your GitHub personal access token with the necessary permissions for repo access.
- OWNER: The GitHub username or organization name that owns the repository.
- REPO: The name of the GitHub repository where the crate data will be stored.
- BRANCH: The branch of the repository where the crate data should be stored.
- PUBLIC_ENDPOINT: The public endpoint where the Private-Crate-Hub will be accessible.
- AUTHORIZATION: An authorization token or method to secure the public endpoint.

Here’s the Docker command:

```bash
docker run -e GITHUB_TOKEN=your_github_token \
           -e OWNER=your_github_username_or_org \
           -e REPO=your_repository_name \
           -e BRANCH=your_branch_name \
           -e PUBLIC_ENDPOINT=https://your-public-endpoint.com \
           -e AUTHORIZATION=your_authorization_token \
           giangndm/private-crate-hub:latest
```

### Using in a Rust Project

1. ** Cargo Configuration **:

```toml
[registry]
global-credential-providers = ["cargo:token"]

[registries]
my-registry = { index = "sparse+http://your-public-endpoint.com/index/" }
```

2. ** Dependencies **:

Add the `registry` field to each private library:

```toml
lib1 = { path = "../../crates/lib1", version = "0.1.2", registry = "my-registry" }
```
