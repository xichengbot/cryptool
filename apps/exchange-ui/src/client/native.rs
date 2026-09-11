use std::{ffi::OsString, path::PathBuf, rc::Rc};

use cryptool::{app::exchange_data::ExchangeDataService, config::Settings};
use directories::ProjectDirs;
use exchange_types::TradeQuery;

use super::{ClientBootstrap, QueryClient, QueryClientHandle, QueryFuture};

pub struct NativeQueryClient {
    service: ExchangeDataService,
}

impl QueryClient for NativeQueryClient {
    fn query_trades(&self, query: TradeQuery) -> QueryFuture {
        let service = self.service.clone();
        Box::pin(async move { service.query(query).await })
    }
}

pub fn bootstrap() -> ClientBootstrap {
    let discovered = ProjectDirs::from("com", "cryptool", "exchange-data")
        .map(|dirs| dirs.config_dir().join("config.toml"));
    let explicit = match parse_config_argument(std::env::args_os()) {
        Ok(path) => path,
        Err(message) => {
            return ClientBootstrap {
                client: None,
                status: message,
            };
        }
    };
    let selected = explicit
        .clone()
        .or_else(|| discovered.as_ref().filter(|path| path.is_file()).cloned());
    let status_path = explicit.as_ref().or(selected.as_ref());
    let settings = match Settings::load(selected.as_deref(), None) {
        Ok(settings) => settings,
        Err(_) => {
            return ClientBootstrap {
                client: None,
                status: safe_config_error(status_path),
            };
        }
    };
    match ExchangeDataService::new(settings) {
        Ok(service) => ClientBootstrap {
            client: Some(QueryClientHandle(Rc::new(NativeQueryClient { service }))),
            status: success_status(selected.as_ref(), discovered.as_ref()),
        },
        Err(_) => ClientBootstrap {
            client: None,
            status: safe_config_error(status_path),
        },
    }
}

fn parse_config_argument(
    args: impl IntoIterator<Item = OsString>,
) -> Result<Option<PathBuf>, String> {
    let mut args = args.into_iter().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--config" {
            return args
                .next()
                .map(PathBuf::from)
                .map(Some)
                .ok_or_else(|| "查询配置不可用：--config 后必须提供配置文件路径".into());
        }
        if let Some(value) = argument.to_string_lossy().strip_prefix("--config=") {
            if value.is_empty() {
                return Err("查询配置不可用：--config 后必须提供配置文件路径".into());
            }
            return Ok(Some(PathBuf::from(value)));
        }
    }
    Ok(None)
}

fn success_status(selected: Option<&PathBuf>, discovered: Option<&PathBuf>) -> String {
    if let Some(path) = selected {
        format!("配置：{}", path.display())
    } else if let Some(path) = discovered {
        format!("使用默认配置；配置文件查找路径：{}", path.display())
    } else {
        "使用默认配置；可通过 --config 指定配置文件".into()
    }
}

fn safe_config_error(path: Option<&PathBuf>) -> String {
    path.map_or_else(
        || "查询配置不可用；请检查应用配置目录中的 config.toml".into(),
        |path| format!("查询配置不可用：{}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::{parse_config_argument, success_status};

    #[test]
    fn config_flag_without_value_is_an_error() {
        let arguments = [OsString::from("exchange-data"), OsString::from("--config")];

        assert!(parse_config_argument(arguments).is_err());
    }

    #[test]
    fn default_status_names_the_discovered_config_path() {
        let discovered = PathBuf::from("/users/test/config/exchange-data/config.toml");

        let status = success_status(None, Some(&discovered));

        assert!(status.contains(discovered.to_string_lossy().as_ref()));
        assert!(status.contains("默认配置"));
    }
}
