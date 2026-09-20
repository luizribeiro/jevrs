use http::{HeaderMap, Method, Uri};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum WasiMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
    Other(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WasiScheme {
    Http,
    Https,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct UrlParts {
    pub(crate) scheme: WasiScheme,
    pub(crate) authority: String,
    pub(crate) path_with_query: String,
}

pub(crate) fn method(method: &Method) -> WasiMethod {
    match *method {
        Method::GET => WasiMethod::Get,
        Method::HEAD => WasiMethod::Head,
        Method::POST => WasiMethod::Post,
        Method::PUT => WasiMethod::Put,
        Method::DELETE => WasiMethod::Delete,
        Method::CONNECT => WasiMethod::Connect,
        Method::OPTIONS => WasiMethod::Options,
        Method::TRACE => WasiMethod::Trace,
        Method::PATCH => WasiMethod::Patch,
        _ => WasiMethod::Other(method.as_str().into()),
    }
}

pub(crate) fn url(uri: &Uri) -> Result<UrlParts, &'static str> {
    let rendered = uri.to_string();
    let scheme = if let Some(scheme) = uri.scheme_str() {
        scheme
    } else {
        let (candidate, remainder) = rendered.split_once(':').ok_or("URL has no scheme")?;
        if candidate.parse::<http::uri::Scheme>().is_err() {
            return Err("URL has no scheme");
        }
        if !remainder.starts_with("//") {
            return Err("URL has no authority");
        }
        candidate
    };
    let authority = uri.authority().ok_or("URL has no authority")?;
    let scheme = match scheme {
        "http" => WasiScheme::Http,
        "https" => WasiScheme::Https,
        _ => return Err("URL has unsupported scheme"),
    };
    let path_with_query = uri.path_and_query().map_or("/", |value| value.as_str());
    Ok(UrlParts {
        scheme,
        authority: authority.as_str().into(),
        path_with_query: path_with_query.into(),
    })
}

pub(crate) fn headers(headers: &HeaderMap) -> Vec<(String, Vec<u8>)> {
    headers
        .iter()
        .map(|(name, value)| (name.as_str().into(), value.as_bytes().into()))
        .collect()
}

#[cfg(test)]
mod tests {
    use http::{HeaderMap, HeaderValue, Method, Uri, header};

    use super::{UrlParts, WasiMethod, WasiScheme, headers, method, url};

    #[test]
    fn splits_default_api_url() {
        let uri: Uri = "https://api.typesafe.ai/v1/systemone".parse().unwrap();

        assert_eq!(
            url(&uri),
            Ok(UrlParts {
                scheme: WasiScheme::Https,
                authority: "api.typesafe.ai".into(),
                path_with_query: "/v1/systemone".into(),
            })
        );
    }

    #[test]
    fn splits_port_path_prefix_and_query() {
        let uri: Uri = "https://localhost:8443/jev/v1/systemone?trace=1"
            .parse()
            .unwrap();

        assert_eq!(
            url(&uri),
            Ok(UrlParts {
                scheme: WasiScheme::Https,
                authority: "localhost:8443".into(),
                path_with_query: "/jev/v1/systemone?trace=1".into(),
            })
        );
    }

    #[test]
    fn splits_http_url() {
        let uri: Uri = "http://127.0.0.1:8080/v1/systemone".parse().unwrap();

        assert_eq!(
            url(&uri),
            Ok(UrlParts {
                scheme: WasiScheme::Http,
                authority: "127.0.0.1:8080".into(),
                path_with_query: "/v1/systemone".into(),
            })
        );
    }

    #[test]
    fn rejects_relative_url() {
        let uri: Uri = "/v1/systemone".parse().unwrap();

        assert_eq!(url(&uri), Err("URL has no scheme"));
    }

    #[test]
    fn rejects_unsupported_scheme() {
        let uri: Uri = "ftp://example.test/v1/systemone".parse().unwrap();

        assert_eq!(url(&uri), Err("URL has unsupported scheme"));
    }

    #[test]
    fn rejects_url_without_authority() {
        let uri: Uri = "mailto:a@b.com".parse().unwrap();

        assert_eq!(url(&uri), Err("URL has no authority"));
    }

    #[test]
    fn maps_standard_and_extension_methods() {
        assert_eq!(method(&Method::POST), WasiMethod::Post);
        assert_eq!(
            method(&Method::from_bytes(b"EVALUATE").unwrap()),
            WasiMethod::Other("EVALUATE".into())
        );
    }

    #[test]
    fn maps_repeated_binary_headers() {
        let mut input = HeaderMap::new();
        input.append(header::ACCEPT, HeaderValue::from_static("application/json"));
        input.append(header::ACCEPT, HeaderValue::from_static("text/plain"));
        input.insert("x-bytes", HeaderValue::from_bytes(b"\x80").unwrap());

        assert_eq!(
            headers(&input),
            vec![
                ("accept".into(), b"application/json".to_vec()),
                ("accept".into(), b"text/plain".to_vec()),
                ("x-bytes".into(), vec![0x80]),
            ]
        );
    }
}
