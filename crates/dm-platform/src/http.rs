//! 极简 HTTP：只够天气接口用。
//!
//! 用 ureq 而不是 reqwest：不需要 async 运行时，二进制也小得多。

use crate::error::{PlatformError, PlatformResult};
use std::time::Duration;

pub fn get_json(url: &str, timeout: Duration) -> PlatformResult<String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .new_agent();

    let mut response = agent
        .get(url)
        .header("User-Agent", "DeskMason/0.1")
        .call()
        .map_err(|error| PlatformError::Http(error.to_string()))?;

    response
        .body_mut()
        .read_to_string()
        .map_err(|error| PlatformError::Http(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_host_returns_an_error() {
        // 不依赖外网：一个保留域名 + 很短的超时。
        let result = get_json("http://127.0.0.1:1/none", Duration::from_millis(200));
        assert!(result.is_err());
    }
}
