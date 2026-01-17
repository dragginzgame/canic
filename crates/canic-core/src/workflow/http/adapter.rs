use crate::{
    dto::http,
    ops::ic::http::{HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult},
};

///
/// HttpAdapter
///

pub struct HttpAdapter;

impl HttpAdapter {
    #[must_use]
    pub fn request_args_from_dto(args: http::HttpRequestArgs) -> HttpRequestArgs {
        HttpRequestArgs {
            url: args.url,
            max_response_bytes: args.max_response_bytes,
            method: Self::method_from_dto(args.method),
            headers: args
                .headers
                .into_iter()
                .map(Self::header_from_dto)
                .collect(),
            body: args.body,
            is_replicated: args.is_replicated,
        }
    }

    #[must_use]
    pub fn result_to_dto(result: HttpRequestResult) -> http::HttpRequestResult {
        http::HttpRequestResult {
            status: result.status,
            headers: result
                .headers
                .into_iter()
                .map(Self::header_to_dto)
                .collect(),
            body: result.body,
        }
    }

    const fn method_from_dto(method: http::HttpMethod) -> HttpMethod {
        match method {
            http::HttpMethod::GET => HttpMethod::Get,
            http::HttpMethod::POST => HttpMethod::Post,
            http::HttpMethod::HEAD => HttpMethod::Head,
        }
    }

    fn header_from_dto(header: http::HttpHeader) -> HttpHeader {
        HttpHeader {
            name: header.name,
            value: header.value,
        }
    }

    fn header_to_dto(header: HttpHeader) -> http::HttpHeader {
        http::HttpHeader {
            name: header.name,
            value: header.value,
        }
    }
}
