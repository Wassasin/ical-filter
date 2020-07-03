use std::net::SocketAddr;

#[derive(Debug)]
pub enum EnvErrorKind {
    Env(std::env::VarError),
    Parse,
}

#[derive(Debug)]
pub struct EnvError<'a>(&'a str, EnvErrorKind);

pub fn var<T: std::str::FromStr>(key: &'static str) -> Result<T, EnvError<'static>> {
    let str = std::env::var(key).map_err(|e| EnvError(key, EnvErrorKind::Env(e)))?;
    str.parse().or(Err(EnvError(key, EnvErrorKind::Parse)))
}

pub fn with_default<'a, T>(
    var_opt: Result<T, EnvError<'a>>,
    default: T,
) -> Result<T, EnvError<'a>> {
    match var_opt {
        Err(EnvError(_, EnvErrorKind::Env(std::env::VarError::NotPresent))) => Ok(default),
        res => res,
    }
}

pub fn get_conf<'a>() -> Result<EnvConfiguration, EnvError<'a>> {
    let socketaddr = with_default(
        var("ICAL_FILTER_SOCKETADDR"),
        "127.0.0.1:8080".parse().unwrap(),
    )?;

    Ok(EnvConfiguration { socketaddr })
}

#[derive(Clone)]
pub struct EnvConfiguration {
    pub socketaddr: SocketAddr,
}
