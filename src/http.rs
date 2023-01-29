use std::{
    fmt::Display,
    fs,
    io::{Error, ErrorKind},
    path::Path,
    str::FromStr,
};

use crate::error::{Result, WebServerError};

fn parse_error(error: String) -> WebServerError {
    WebServerError::IOError(Error::new(ErrorKind::InvalidData, error))
}

pub type Uri = String;
pub type HttpVersion = String;

pub const HTTP_VERSION: &str = "HTTP/1.1";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Charset {
    Utf8,
}

impl Display for Charset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "utf-8")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ContentType {
    TextHTML(Option<Charset>),
    TextJavascript(Option<Charset>),
    ApplicationWASM,
}

impl ContentType {
    pub fn html() -> Self {
        Self::TextHTML(None)
    }
    pub fn javascript() -> Self {
        Self::TextJavascript(None)
    }

    fn from_file_path_opt(filename: &Path) -> Option<Self> {
        match filename.extension()?.to_str()? {
            "html" => Some(Self::html()),
            "js" => Some(Self::javascript()),
            "wasm" => Some(Self::ApplicationWASM),
            _ => None,
        }
    }

    pub fn from_file_path(filename: &Path) -> Result<Self> {
        Self::from_file_path_opt(filename).ok_or(WebServerError::Other(format!(
            "File extension not recognized for {:?}",
            filename
        )))
    }
}

impl Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextHTML(None) => write!(f, "text/html"),
            Self::TextHTML(Some(c)) => write!(f, "text/html charset:{}", c),
            Self::TextJavascript(None) => write!(f, "text/javascript"),
            Self::TextJavascript(Some(c)) => write!(f, "text/javascript charset:{}", c),
            Self::ApplicationWASM => write!(f, "application/wasm"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Header {
    Host(String),
    ContentLength(usize),
    ContentType(ContentType),
    Other(String, String),
}

impl FromStr for Header {
    type Err = WebServerError;

    fn from_str(line: &str) -> Result<Self> {
        let (header, contents) = line.split_once(':').ok_or(parse_error(format!(
            "Incorrectly formatted header: {} length: {}",
            line,
            line.len(),
        )))?;
        let contents = &contents[1..]; // drop space
        match header {
            "Host" => Ok(Header::Host(String::from(contents))),
            _ => Ok(Header::Other(String::from(header), String::from(contents))),
        }
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host(s) => write!(f, "Host: {}", s),
            Self::ContentLength(n) => write!(f, "Content-Length: {}", n),
            Self::ContentType(c) => write!(f, "Content-Type: {}", c),
            Self::Other(s, t) => write!(f, "{}: {}", s, t),
        }
    }
}

#[derive(Clone, Debug)]
pub enum RequestMethod {
    Head,
    Get,
    Post(String),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Request {
    uri: Uri,
    http_version: HttpVersion,
    headers: Vec<Header>,
    method: RequestMethod,
}

impl Request {
    fn parse_request_headers_and_data(request: Vec<&str>) -> Result<(Vec<Header>, Option<String>)> {
        let mut current_line = 1_usize; // skip first line
        let mut headers = Vec::new();
        while current_line < request.len()
            && request[current_line] != "\r\n"
            && request[current_line] != ""
        {
            headers.push(Header::from_str(request[current_line])?);
            current_line += 1;
        }

        let data = request.get(current_line + 1..).map(|v| v.concat());

        Ok((headers, data))
    }

    pub fn raw(uri: &str, http_version: &str, headers: &[Header], method: RequestMethod) -> Self {
        Request {
            uri: String::from(uri),
            http_version: String::from(http_version),
            headers: headers.to_owned(),
            method,
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn http_version(&self) -> &str {
        &self.http_version
    }

    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    pub fn request_method(&self) -> &RequestMethod {
        &self.method
    }
}
impl Request {
    pub fn parse(request: &str) -> Result<Self> {
        let request_lines: Vec<&str> = request.split("\r\n").collect();

        let first_line: Vec<&str> = request_lines
            .get(0)
            .ok_or(parse_error(format!("Request empty")))?
            .split(" ")
            .collect();

        if first_line.len() != 3 {
            return Result::Err(parse_error(format!("Malformed request: {}", request)));
        }

        let uri: Uri = String::from(first_line[1]);
        let http_version: HttpVersion = String::from(first_line[2]);

        let (headers, data) = Request::parse_request_headers_and_data(request_lines)?;

        let method = match first_line[0] {
            "GET" => Ok(RequestMethod::Get),
            "POST" => {
                let data = data.ok_or(parse_error(format!("Post missing data")))?;
                Ok(RequestMethod::Post(data))
            }
            _ => Result::Err(parse_error(format!(
                "Failed to match request type with: {}",
                first_line[0]
            ))),
        }?;

        Ok(Request {
            uri,
            http_version,
            headers,
            method,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Ok,
    NotFound,
    Forbidden,
    MethodNotAllowed,
    InternalServerError,
}

impl ResponseType {
    fn condition_code(&self) -> usize {
        match self {
            Self::Ok => 200,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::InternalServerError => 500,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Ok => "Ok",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::InternalServerError => "Internal Server Error",
        }
    }
}

impl ToString for ResponseType {
    fn to_string(&self) -> String {
        let condition_code = self.condition_code();
        let name = self.name();
        format!("{} {}", condition_code, name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    which: ResponseType,
    http_version: HttpVersion,
    headers: Vec<Header>,
    data: Option<Vec<u8>>,
}

impl Response {
    pub fn to_raw(&self) -> Vec<u8> {
        let headers = self
            .headers
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<String>>()
            .join("\r\n");
        match self.data.to_owned() {
            Some(data) => format!(
                "{} {}\r\n{}\r\n\r\n",
                self.http_version,
                self.which.to_string(),
                headers,
            )
            .as_bytes()
            .iter()
            .chain(&data)
            .copied()
            .collect(),
            None => format!(
                "{} {}\r\n{}",
                self.http_version,
                self.which.to_string(),
                headers,
            )
            .as_bytes()
            .to_vec(),
        }
    }

    pub fn serve_file(
        filename: &Path,
        content_type: ContentType,
        response_type: ResponseType,
    ) -> Result<Self> {
        let data = fs::read(filename)?;

        let headers = vec![
            Header::ContentLength(data.len()),
            Header::ContentType(content_type),
        ];

        Ok(Response {
            which: response_type,
            http_version: String::from(HTTP_VERSION),
            headers,
            data: Some(data),
        })
    }

    pub fn html_page(filename: &Path) -> Result<Self> {
        Self::serve_file(filename, ContentType::html(), ResponseType::Ok)
    }

    pub fn empty_internal_server_error() -> Self {
        let html = "
<!DOCTYPE html>
<html lang=\"en\">

<head>
    <meta charset=\"utf-8\">
    <title>Internal Server Error (500)</title>
</head>

<body>
    <h1>Internal Server Error (500)</h1>
    <p>Something went wrong! The server is confused.</p>
</body>

</html>";
        Response {
            which: ResponseType::InternalServerError,
            http_version: format!("{}", HTTP_VERSION),
            headers: vec![
                Header::ContentLength(html.len()),
                Header::ContentType(ContentType::html()),
            ],
            data: Some(Vec::from(html)),
        }
    }

    pub fn to_head(&self) -> Self {
        Response {
            which: self.which,
            http_version: self.http_version.clone(),
            headers: self.headers.clone(),
            data: None,
        }
    }
}
