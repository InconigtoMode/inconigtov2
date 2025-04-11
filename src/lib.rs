mod common;
mod config;
mod proxy;

use crate::config::Config;
use crate::proxy::*;

use std::collections::HashMap;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;
use worker::*;
use once_cell::sync::Lazy;
use regex::Regex;

static PROXYIP_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^.+-\d+$").unwrap());

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let uuid = env
        .var("UUID")
        .map(|x| Uuid::parse_str(&x.to_string()).unwrap_or_default())?;
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    let main_page_url = env.var("MAIN_PAGE_URL").map(|x|x.to_string()).unwrap();
    let sub_page_url = env.var("SUB_PAGE_URL").map(|x|x.to_string()).unwrap();
    let config = Config { uuid, host: host.clone(), proxy_addr: host, proxy_port: 443, main_page_url, sub_page_url};

    Router::with_data(config)
        .on_async("/", fe)
        .on_async("/sub", sub)
        .on("/link", link)
        .on_async("/:proxyip", tunnel)
        .on_async("/Inconigto-Mode/:proxyip", tunnel)
        .run(req, env)
        .await
}

async fn get_response_from_url(url: String) -> Result<Response> {
    let req = Fetch::Url(Url::parse(url.as_str())?);
    let mut res = req.send().await?;
    Response::from_html(res.text().await?)
}

async fn fe(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    get_response_from_url(cx.data.main_page_url).await
}

async fn sub(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    get_response_from_url(cx.data.sub_page_url).await
}


async fn tunnel(req: Request, mut cx: RouteContext<Config>) -> Result<Response> {
    let mut proxyip = cx.param("proxyip").unwrap().to_string();
    if proxyip.len() == 2 {
        let req = Fetch::Url(Url::parse("https://raw.githubusercontent.com/FoolVPN-ID/Nautica/refs/heads/main/kvProxyList.json")?);
        let mut res = req.send().await?;
        if res.status_code() == 200 {
            let proxy_kv: HashMap<String, Vec<String>> = serde_json::from_str(&res.text().await?)?;
            proxyip = proxy_kv[&proxyip][0].clone().replace(":", "-");
        }
    }

    if PROXYIP_PATTERN.is_match(&proxyip) {
        if let Some((addr, port_str)) = proxyip.split_once('-') {
            if let Ok(port) = port_str.parse() {
                cx.data.proxy_addr = addr.to_string();
                cx.data.proxy_port = port;
            }
        }
    }
    
    let upgrade = req.headers().get("Upgrade")?.unwrap_or("".to_string());
    if upgrade == "websocket".to_string() {
        let WebSocketPair { server, client } = WebSocketPair::new()?;
        server.accept()?;
    
        wasm_bindgen_futures::spawn_local(async move {
            let events = server.events().unwrap();
            if let Err(e) = ProxyStream::new(cx.data, &server, events).process().await {
                console_log!("[tunnel]: {}", e);
            }
        });
    
        Response::from_websocket(client)
    } else {
        Response::from_html("https://inconigto-mode.web.id/")
    }

}

fn link(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    // You can extract any context data you need here
    // For example:
    let host = cx.data.host.to_string();
    
    // Create an HTML response string with basic structure
    let html = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Connection Links</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
        }}
        body {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
            color: #333;
        }}
        .container {{
            max-width: 800px;
            width: 100%;
            background-color: white;
            border-radius: 16px;
            box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
            overflow: hidden;
        }}
        .header {{
            background: linear-gradient(90deg, #3b82f6 0%, #2563eb 100%);
            padding: 30px;
            color: white;
            text-align: center;
        }}
        h1 {{
            font-weight: 700;
            font-size: 28px;
            margin-bottom: 10px;
        }}
        .subtitle {{
            font-size: 16px;
            opacity: 0.9;
        }}
        .content {{
            padding: 30px;
        }}
        .server-info {{
            background-color: #f8fafc;
            border-radius: 12px;
            padding: 20px;
            margin-top: 20px;
            border-left: 4px solid #3b82f6;
            display: flex;
            align-items: center;
        }}
        .icon {{
            margin-right: 15px;
            width: 40px;
            height: 40px;
            background-color: #dbeafe;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .server-name {{
            font-size: 18px;
            font-weight: 600;
            color: #1e40af;
        }}
        .status {{
            display: inline-block;
            background-color: #dcfce7;
            color: #166534;
            font-size: 14px;
            font-weight: 500;
            padding: 4px 12px;
            border-radius: 20px;
            margin-top: 10px;
        }}
        .footer {{
            text-align: center;
            padding: 20px;
            border-top: 1px solid #e2e8f0;
            color: #64748b;
            font-size: 14px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Connection Information</h1>
            <div class="subtitle">Secure server connection details</div>
        </div>
        <div class="content">
            <p>Welcome to your server connection portal. Below you'll find all the information needed to establish a secure connection.</p>
            
            <div class="server-info">
                <div class="icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect>
                        <rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect>
                        <line x1="6" y1="6" x2="6.01" y2="6"></line>
                        <line x1="6" y1="18" x2="6.01" y2="18"></line>
                    </svg>
                </div>
                <div>
                    <div class="server-name">{}</div>
                    <div class="status">Active</div>
                </div>
            </div>
        </div>
        <div class="footer">
            &copy; {{"2024"}} Connection Manager • All rights reserved
        </div>
    </div>
</body>
</html>
"#,
        host
    );

    // Return HTML response
    Response::from_html(html)
}
