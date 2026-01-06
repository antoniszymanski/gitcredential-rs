// SPDX-FileCopyrightText: 2026 Antoni Szymański
// SPDX-License-Identifier: MPL-2.0

use snafu::{ResultExt, Snafu};
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(feature = "url")]
use url::Url;

#[derive(Debug, Default)]
pub struct GitCredential {
    /// The protocol over which the credential will be used (e.g., https).
    pub protocol: Option<String>,
    /// The remote hostname for a network credential. This includes the port number if one was specified (e.g., "example.com:8088").
    pub host: Option<String>,
    /// The path with which the credential will be used. E.g., for accessing a remote https repository, this will be the repository’s path on the server.
    pub path: Option<String>,
    /// The credential’s username, if we already have one (e.g., from a URL, the configuration, the user, or from a previously run helper).
    pub username: Option<String>,
    /// The credential’s password, if we are asking it to be stored.
    pub password: Option<String>,
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[non_exhaustive]
pub enum FromReaderError {
    #[snafu(display("Failed to read line from input reader"))]
    ReadLine { source: io::Error },
    #[snafu(display("Line exceeds {MAX_LINE_LENGTH} bytes limit"))]
    TooLongLine,
    #[snafu(display("Failed to parse line (expected a key-value pair): {line:?}"))]
    InvalidLine { line: String },
    #[cfg(feature = "url")]
    #[snafu(display("Failed to parse URL: {input:?}"))]
    InvalidUrl { source: url::ParseError, input: String },
}

const MAX_LINE_LENGTH: usize = 65535 - 1;

impl GitCredential {
    pub fn from_reader(reader: impl Read) -> Result<Self, FromReaderError> {
        let mut gc = Self::default();
        let buf_reader = BufReader::new(reader);
        for line in buf_reader.lines() {
            let line = line.context(ReadLineCtx)?;
            if line.is_empty() {
                break;
            } else if line.len() > MAX_LINE_LENGTH {
                return Err(FromReaderError::TooLongLine);
            }
            let (key, value) = match line.split_once('=') {
                Some(v) => v,
                None => return Err(FromReaderError::InvalidLine { line }),
            };
            match key {
                "protocol" => put_str(&mut gc.protocol, value),
                "host" => put_str(&mut gc.host, value),
                "path" => put_str(&mut gc.path, value),
                "username" => put_str(&mut gc.username, value),
                "password" => put_str(&mut gc.password, value),
                #[cfg(feature = "url")]
                "url" => gc.set_url(&Url::parse(value).context(InvalidUrlCtx { input: value })?),
                _ => {}
            }
        }
        Ok(gc)
    }

    pub fn to_writer(&self, mut writer: impl Write) -> Result<(), io::Error> {
        if let Some(protocol) = &self.protocol {
            writeln!(writer, "protocol={protocol}")?;
        }
        if let Some(host) = &self.host {
            writeln!(writer, "host={host}")?;
        }
        if let Some(path) = &self.path {
            writeln!(writer, "path={path}")?;
        }
        if let Some(username) = &self.username {
            writeln!(writer, "username={username}")?;
        }
        if let Some(password) = &self.password {
            writeln!(writer, "password={password}")?;
        }
        Ok(())
    }

    #[cfg(feature = "url")]
    pub fn from_url(url: &Url) -> Self {
        let mut gc = Self::default();
        gc.set_url(url);
        gc
    }

    #[cfg(feature = "url")]
    pub fn set_url(&mut self, url: &Url) {
        put_str(&mut self.protocol, url.scheme());
        put_opt_str(&mut self.host, url.host_str());
        put_str(&mut self.path, trim_prefix(url.path(), "/"));
        put_opt_str(&mut self.username, Some(url.username()).filter(|s| !s.is_empty()));
        put_opt_str(&mut self.password, url.password());
    }
}

#[inline]
fn put_opt_str(dst: &mut Option<String>, src: Option<&str>) {
    match src {
        Some(src) => put_str(dst, src),
        None => *dst = None,
    }
}

#[inline]
fn put_str(dst: &mut Option<String>, src: &str) {
    match dst {
        Some(dst) => src.clone_into(dst),
        None => *dst = Some(src.to_owned()),
    }
}

#[inline]
fn trim_prefix<'a>(s: &'a str, prefix: &'a str) -> &'a str {
    s.strip_prefix(prefix).unwrap_or(s)
}
