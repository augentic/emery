use tracing::level_filters::LevelFilter;

#[cfg_attr(
    not(any(target_arch = "wasm32", test)),
    expect(dead_code, reason = "called from the component boundary, which exists on wasm32 alone")
)]
pub fn directives(level: LevelFilter, adapter: &str) -> String {
    if level == LevelFilter::DEBUG {
        format!("info,emery_sdk=debug,omnia_sdk=debug,{adapter}=debug")
    } else if level == LevelFilter::TRACE {
        format!("debug,emery_sdk=trace,omnia_sdk=trace,{adapter}=trace")
    } else {
        level.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets() {
        for (level, expected) in [
            (LevelFilter::OFF, "off"),
            (LevelFilter::ERROR, "error"),
            (LevelFilter::WARN, "warn"),
            (LevelFilter::INFO, "info"),
            (LevelFilter::DEBUG, "info,emery_sdk=debug,omnia_sdk=debug,emery_intent=debug"),
            (LevelFilter::TRACE, "debug,emery_sdk=trace,omnia_sdk=trace,emery_intent=trace"),
        ] {
            assert_eq!(directives(level, "emery_intent"), expected);
        }
    }
}
