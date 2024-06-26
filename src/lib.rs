use std::fmt::{Display, Formatter};
use std::str::FromStr;

// pub use haml::*;
pub mod manifested_schema;
pub mod haml_parser;

#[derive(Debug, Default, Clone)]
pub struct Location {
    pub file_name: String,
    pub line: u64,
    pub column: u64,
    pub child_index: u64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum CoreApi {
    Register,
    LoginByEmail,
    LoginByUsername,
    OAuth,
    PasswordResetTrigger,
    PasswordReset,
    MagicLink,
    TwoFactorAuthEmail,
    TwoFactorAuthSms,
    TwoFactorStep2,
    TwoFactorTotp,
    VerifyAccount,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DatabaseType {
    MekaDb,
    Postgres,
    MySQL,
    MariaDB,
    Oracle,
    MsSql,
}

impl DatabaseType {
    pub fn from(v: &String) -> Option<DatabaseType> {
        match v.to_lowercase().as_str() {
            "mekadb" => Some(DatabaseType::MekaDb),
            "postgres" => Some(DatabaseType::Postgres),
            "mysql" => Some(DatabaseType::MySQL),
            "mariadb" => Some(DatabaseType::MariaDB),
            "oracle" => Some(DatabaseType::Oracle),
            "mssql" => Some(DatabaseType::MsSql),
            _ => None,
        }
    }
}

impl Display for DatabaseType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseType::MekaDb => f.write_str("MekaDb"),
            DatabaseType::Postgres => f.write_str("Postgres"),
            DatabaseType::MySQL => f.write_str("MySQL"),
            DatabaseType::MariaDB => f.write_str("MariaDB"),
            DatabaseType::Oracle => f.write_str("Oracle"),
            DatabaseType::MsSql => f.write_str("MsSql"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConstraintViolationAction {
    Cascade,
    Restrict,
}

#[derive(Debug, Clone)]
pub enum TableConstraintType {
    ForeignKey {
        on_delete: Option<ConstraintViolationAction>,
        on_update: Option<ConstraintViolationAction>,
    },
    Unique,
}

#[derive(Debug, Clone)]
pub enum ImplicitDockerStepPosition {
    First,
    Each,
    Last,
}

impl FromStr for ImplicitDockerStepPosition {
    type Err = String;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "first" => Ok(ImplicitDockerStepPosition::First),
            "each" => Ok(ImplicitDockerStepPosition::Each),
            "last" => Ok(ImplicitDockerStepPosition::Each),
            _ => Err(format!("Invalid position '{}'", input)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DockerConnectionInfo {
    pub start_pos: Location,
    pub end_pos: Location,
    pub username: Option<String>,
    pub password: Option<String>,
    pub image: String,
    pub tag: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DockerStepProvider {
    Custom { name: String, path: String },
    Dockerfile { path: String },
    DockerImage(DockerConnectionInfo),
    Remote { host: String, port: Option<String> },
}

impl FromStr for DockerStepProvider {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.to_lowercase();
        if input.ends_with("dockerfile") {
            input
                .strip_prefix("file:")
                .unwrap_or("")
                .strip_suffix("dockerfile")
                .map(|v| DockerStepProvider::Dockerfile {
                    path: v.strip_prefix("/").unwrap_or(v).strip_suffix("/").unwrap_or(v).to_string(),
                })
                .ok_or_else(|| "Unable to parse plugin provider as a Dockerfile source".to_string())
        } else if input.starts_with("hypi:") {
            let input = input.strip_prefix("hypi:").unwrap();
            Ok(DockerStepProvider::DockerImage(parse_docker_image(input)?))
        } else if input.starts_with("remote:") {
            let input = input.strip_prefix("remote:").unwrap();
            let idx = input.find(":");
            Ok(DockerStepProvider::Remote {
                host: input[0..idx.unwrap_or(input.len())].to_string(),
                port: idx.map(|idx| input[idx + 1..].to_string()),
            })
        } else if input.starts_with("docker:") {
            let input = input.strip_prefix("docker:").unwrap();
            Ok(DockerStepProvider::DockerImage(parse_docker_image(input)?))
        } else {
            if input.contains(":") {
                let builder_name = input.chars().take_while(|c| c != &':');
                let path = input.split(":").last().unwrap().to_owned();
                Ok(DockerStepProvider::Custom {
                    name: builder_name.collect(),
                    path,
                })
            } else {
                Err(format!("Unsupported step provider '{}'", input))
            }
        }
    }
}

pub fn parse_docker_image(input: &str) -> Result<DockerConnectionInfo, String> {
    let (username, pass, image, tag) = if input.contains("@") {
        let mut parts = input.split("@");
        let user_and_pass = parts
            .next()
            .ok_or_else(|| "Provider with @ must be in the form user:pass@image:tag".to_string())?;
        let mut user_and_pass = user_and_pass.split(":");
        let user = user_and_pass
            .next()
            .ok_or_else(|| "Provider with @ must be in the form user:pass@image:tag".to_string())?;
        let pass = user_and_pass
            .next()
            .ok_or_else(|| "Provider with @ must be in the form user:pass@image:tag".to_string())?;
        let image_and_tag = parts
            .next()
            .ok_or_else(|| "Provider with @ must be in the form user:pass@image:tag".to_string())?;
        let img = image_and_tag.chars().take_while(|v| v != &':').collect();
        let tag = image_and_tag.split(":").last().map(|v| v.to_owned());
        (Some(user), Some(pass), Some(img), Some(tag))
    } else {
        (None, None, None, None)
    };
    Ok(DockerConnectionInfo {
        start_pos: Default::default(),
        end_pos: Default::default(),
        username: username.map(|v| v.to_owned()),
        password: pass.map(|v| v.to_owned()),
        image: if let Some(img) = image {
            img
        } else if input.contains(":") {
            input.chars().take_while(|v| v != &':').collect()
        } else {
            input.to_owned()
        },
        tag:
        if let Some(v) = tag {
            v
        } else {
            if input.contains(":") {
                input.split(":").last().map(|v| v.to_owned())
            } else {
                None
            }
        }
        ,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_parse_builtin_plugins() -> Result<(), String> {
        match "hypi:form".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "form");
                assert_eq!(info.tag, None);
            }
            _ => panic!("should've gotten a docker image")
        }
        match "hypi:hypi/rapid-plugin-form".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "hypi/rapid-plugin-form");
                assert_eq!(info.tag, None);
            }
            _ => panic!("should've gotten a docker image")
        }
        match "hypi:repo.hypi.ai/rapid-plugin-form".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "repo.hypi.ai/rapid-plugin-form");
                assert_eq!(info.tag, None);
            }
            _ => panic!("should've gotten a docker image")
        }
        match "hypi:repo.hypi.ai/rapid-plugin-form:v2".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "repo.hypi.ai/rapid-plugin-form");
                assert_eq!(info.tag, Some("v2".to_string()));
            }
            _ => panic!("should've gotten a docker image")
        }
        match "hypi:user2:pass2@repo.hypi.ai/rapid-plugin-form:v2".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "repo.hypi.ai/rapid-plugin-form");
                assert_eq!(info.tag, Some("v2".to_string()));
                assert_eq!(info.username, Some("user2".to_string()));
                assert_eq!(info.password, Some("pass2".to_string()));
            }
            _ => panic!("should've gotten a docker image")
        }
        match "hypi:user3:pass3@hypi/rapid-plugin-form:v3".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "hypi/rapid-plugin-form");
                assert_eq!(info.tag, Some("v3".to_string()));
                assert_eq!(info.username, Some("user3".to_string()));
                assert_eq!(info.password, Some("pass3".to_string()));
            }
            _ => panic!("should've gotten a docker image")
        }
        match "docker:form".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "form");
                assert_eq!(info.tag, None);
            }
            _ => panic!("should've gotten a docker image")
        }
        match "hypi:form:latest".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "form");
                assert_eq!(info.tag, Some("latest".to_string()));
            }
            _ => panic!("should've gotten a docker image")
        }
        match "hypi:form:v123".parse()? {
            DockerStepProvider::DockerImage(info) => {
                assert_eq!(info.image, "form");
                assert_eq!(info.tag, Some("v123".to_string()));
            }
            _ => panic!("should've gotten a docker image")
        }
        match "file:my-plugin/Dockerfile".parse()? {
            DockerStepProvider::Dockerfile { path } => {
                assert_eq!(path, "my-plugin");
            }
            _ => panic!("should've gotten a docker image")
        }
        match "remote:localhost:2020".parse()? {
            DockerStepProvider::Remote { host, port } => {
                assert_eq!(host, "localhost");
                assert_eq!(port, Some(2020.to_string()));
            }
            _ => panic!("should've gotten a remote host and port")
        }
        match "remote:localhost".parse()? {
            DockerStepProvider::Remote { host, port } => {
                assert_eq!(host, "localhost");
                assert_eq!(port, None);
            }
            _ => panic!("should've gotten a docker image")
        }
        Ok(())
    }
}
