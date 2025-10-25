use ammonia::Builder;
use axum::http::HeaderValue;
use deunicode;
use maplit::hashset;
use std::str::FromStr;
use templr::Trust;
use templr::{templ, templ_ret};
use url::Url;

pub fn simplify_html(input: String, base: String) -> anyhow::Result<String> {
    let tags = hashset![
        "a",
        "br",
        "ol",
        "li",
        "p",
        "small",
        "font",
        "b",
        "strong",
        "i",
        "em",
        "blockquote",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "img"
    ];

    let mut readability = readable_readability::Readability::new();
    readability.base_url(Url::from_str(&base)?);
    readability.clean_attributes(true);
    readability.strip_unlikelys(true);
    readability.clean_conditionally(false);
    readability.weight_classes(false);

    let (content, meta) = readability.parse(&input);

    let mut content_bytes = vec![];
    content
        .serialize(&mut content_bytes)
        .map_err(|e| anyhow::anyhow!(format!("Cannot serialize content: {e}")))?;
    let content = std::str::from_utf8(&content_bytes)
        .map_err(|e| anyhow::anyhow!(format!("Can't make string from content. Error: {e}")))?;

    let result = Builder::new()
        .tags(tags)
        .link_rel(None)
        .clean(content)
        .to_string();

    let result = format!(
        "<h1>{}</h1>{}",
        meta.page_title.unwrap_or("Untitled page".to_string()),
        result
    );

    Ok(deunicode::deunicode(&result).to_string())
}

pub fn replacements(input: String, base_path: String) -> String {
    // Replacing url to proxy
    let r = input.replace("href=\"http", &format!("href=\"{base_path}?url=http"));
    // Some minor compatibility adaptations
    let r = r.replace("strong>", "b>");

    r.replace("em>", "i>")
}

pub async fn process_page(page: String, base_path: String) -> anyhow::Result<String> {
    let url = Url::from_str(&page)?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.append(
        "User-Agent",
        HeaderValue::from_str("Mozilla/5.0 (compatible; IBrowse 3.0; AmigaOS4.0)")?,
    );

    let body = reqwest::Client::builder()
        .default_headers(headers)
        .build()?
        .get(url.clone())
        .send()
        .await?
        .text()
        .await?;

    let simplified = simplify_html(body, url.to_string())?;

    let ready = replacements(simplified, base_path);

    Ok(ready)
}

pub fn proxy_page(path: String, content: String) -> templ_ret!['static] {
    templ! {
        <html>
            <head>
                <title>BoingSearch Simplifier</title>
            </head>
            <body>
            <form action="/browse/" method="get">
                <a href="/">Back to the root!</a> | Current URL:
                <input type="text" size="30" name="url" value={path}/> <input type="submit" value="Go!"/>
            </form>
            <hr/>

            {Trust(content.clone())}

            </body>
        </html>
    }
}

#[cfg(test)]
mod tests {
    use crate::server::simplifier::process_page;
    use std::io::Write;

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let page = process_page(
            "https://amigaforever.com/".to_string(),
            "http://boingsearch.com/browse/".to_string(),
        )
        .await?;

        let mut file = std::fs::File::create("test.html")?;
        write!(file, "{page}");

        Ok(())
    }
}
