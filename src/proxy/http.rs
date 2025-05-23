use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};

use crate::proxy::runtime::ProxyRuntime;

pub async fn start_http_server(
    runtime: Arc<ProxyRuntime>,
    group_name: String,
    candidates: Vec<String>,
) {
    use hyper::{Body, Request, Response, Server, Method, StatusCode};
    use hyper::service::{make_service_fn, service_fn};
    use std::convert::Infallible;
    use std::net::SocketAddr;

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let make_svc = make_service_fn(move |_| {
        let runtime = runtime.clone();
        let group_name = group_name.clone();
        let candidates = candidates.clone();

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle(
                    req,
                    runtime.clone(),
                    group_name.clone(),
                    candidates.clone(),
                )
            }))
        }
    });

    println!("[HTTP] Listening on http://{}", addr);
    Server::bind(&addr).serve(make_svc).await.unwrap();
}

async fn handle(
    req: Request<Body>,
    runtime: Arc<ProxyRuntime>,
    group_name: String,
    candidates: Vec<String>,
) -> Result<Response<Body>, Infallible> {
    let group = runtime.get_group(&group_name);
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/proxies") => {
            if let Some(group) = runtime.get_group(&group_name) {
                let current = group.get();
                let mut response = format!("Current: {}\n\nAvailable:\n", current);

                for p in &candidates {
                    if *p == current {
                        response.push_str(&format!("- ✅ {}\n", p));
                    } else {
                        response.push_str(&format!("- {}\n", p));
                    }
                }

                return Ok(utf8_response(response));
            } else {
                return Ok(Response::new(Body::from("No such group\n")));
            }
        }

        (&Method::GET, "/proxy") | (&Method::POST, "/proxy") => {
            if let Some(group) = runtime.get_group(&group_name) {
                if let Some(query) = req.uri().query() {
                    let params = url::form_urlencoded::parse(query.as_bytes()).collect::<Vec<_>>();
                    for (key, value) in params {
                        if key == "to" {
                            println!("[HTTP] Switching Proxy to {}", value);
                            group.set(&value);

                            // ✅ 如果是浏览器点击，跳转回 /ui
                            if req.method() == Method::GET {
                                let mut resp = Response::new(Body::empty());
                                *resp.status_mut() = StatusCode::FOUND;
                                resp.headers_mut().insert(
                                    hyper::header::LOCATION,
                                    "/ui".parse().unwrap(),
                                );
                                return Ok(resp);
                            }

                            // curl POST 响应
                            return Ok(Response::new(Body::from(format!("Switched to: {}\n", value))));
                        }
                    }
                }
                Ok(Response::new(Body::from("Missing ?to=xxx\n")))
            } else {
                Ok(Response::new(Body::from("No such group\n")))
            }
        }

        (&Method::GET, "/ui") => {
            let group = runtime.get_group(&group_name);
            if let Some(group) = group {
                let current = group.get();
                let mut html = String::new();
                html.push_str("<html><head><meta charset='utf-8'><title>Proxy Switcher</title></head><body>");
                html.push_str("<h2>🚀 当前代理节点</h2>");
                html.push_str(&format!("<p><b>{}</b></p>", current));

                html.push_str("<h3>🧭 可选节点：</h3><ul>");
                for p in &candidates {
                    html.push_str(&format!(
                        "<li><a href='/proxy?to={}'>{}</a></li>",
                        urlencoding::encode(p),  // URL 编码中文
                        if *p == current { format!("✅ {}", p) } else { p.to_string() }
                    ));
                }
                html.push_str("</ul></body></html>");

                let mut resp = Response::new(Body::from(html));
                resp.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    "text/html; charset=utf-8".parse().unwrap(),
                );
                return Ok(resp);
            } else {
                return Ok(Response::new(Body::from("No proxy group")));
            }
        }

        _ => {
            Ok(Response::new(Body::from("Not Found\n")))
        }
    }
}

fn utf8_response(text: impl Into<String>) -> Response<Body> {
    let mut resp = Response::new(Body::from(text.into()));
    resp.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        "text/plain; charset=utf-8".parse().unwrap(),
    );
    resp
}