# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/giangndm/private-crate-hub/compare/v0.1.0...v0.1.1) - 2024-08-12

### Added
- allow get crate meta without auth
- allow unauth with config.info
- allow disable auth
- allowed config http port

### Fixed
- return 401 instead of 403
- temp fix auth error with release-plz by disable metadata api auth. TODO: re-enable it
- return 404 if crate not found
- clap params conflic

### Other
- install missed packages with docker release
- update README
- split lib and store in same layout with get api
- allow build dev version
