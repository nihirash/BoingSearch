use templr::{Template, templ, templ_ret};

use crate::server::search::SearchResponse;
use crate::server::search::Serp;
use deunicode::deunicode;

pub fn serp_result_page(query: String, serp_result: SearchResponse) -> anyhow::Result<String> {
    let template = templ! {
        <html>
            <head>
                <title>BoingSearch! {query} </title>
            </head>
            <body>
                <form action="/" method= "get">
                    <table widht="100%" border="0">
                        <tr widht="100%">
                            <td>
                                <a href="/"><img src="/static/logo.gif" alt="BoingSearch Logo" /></a>
                            </td>
                            <td>
                                    Search results for: <input type="text" size="30" name="q" value={query}/><br/><br/>
                                    #if serp_result.inputs.contains_key("premium") {
                                        <input type="checkbox" name="premium" checked /> Use SerpAPI for search | <input type="submit" value="Search!"/><br/>
                                    } else {
                                        <input type="checkbox" name="premium" /> Use SerpAPI for search | <input type="submit" value="Search!"/><br/>
                                    }
                            </td>
                        </tr>
                    </table>
                </form>

                <hr/>

                    #for item in &serp_result.serp {
                        #render_serp_item(item.clone());
                        <br/>
                    }
                #build_footer();
            </body>
            </html>
    };

    template.render(&())
}

pub fn build_home_page(serpapi_left: u64) -> anyhow::Result<String> {
    let template = templ! {
        <html>
        <head>
            <title>BoingSearch!</title>
        </head>
        <body>

            <br/>
            <br/>
            <center><img src="/static/logo.gif" alt="BoingSearch Logo"/></center>

            <center>
                <h2>The Search Engine for Amigans and Friends</h2>
                <h3>And web page <a href="/browse/">simplificator</a></h3>
            </center>

            <center>
                <form action="/" method="get">
                I am looking for: <br/>
                    <input type="text" size="30" name="q"/> <br/>
                    #if serpapi_left > 0 {
                        <input type="checkbox" name="premium" /> Use SerpAPI(limited count per month) for search <br/>
                        Left {serpapi_left} search queries on SerpAPI account
                        <small>SerpAPI uses Google as source, in other case - we are using DuckDuckGo</small> <br/>
                    }

                    <input type="submit" value="Search!"/>
                </form>
            </center>

            #build_footer();
        </body>
        </html>
    };

    template.render(&())
}

pub fn build_error_page(message: String) -> anyhow::Result<String> {
    let template = templ! {
        <html>
        <head>
            <title>BoingSearch!</title>
        </head>
        <body>

            <br/>
            <br/>
            <center><a href="/"><img src="/static/logo.gif" alt="BoingSearch Logo"/></a></center>
            <br/>
            <center><h2>Warning</h2></center>

            <center>{message}</center>

            #build_footer();
        </body>
        </html>
    };

    Ok(template.render(&())?.to_string())
}

fn render_serp_item(serp_item: Serp) -> templ_ret!['static] {
    templ! {
        <h3>{serp_item.title}</h3>
        <h4>{serp_item.displayed_link}</h4>
        <a href={format!("/browse/?url={}", serp_item.link.clone())}>[Simplified page]</a> |
        <a href={serp_item.link}>[Full version]</a><br/>
        <small>
            {deunicode(&serp_item.snippet.clone().unwrap_or("".to_string()))}
        </small>
        <hr/>
    }
}

fn build_footer() -> templ_ret!['static] {
    templ! {
            <br/>
            <br/>
            <center>Changelog available <a href="/static/changelog.html">here</a></center>
            <hr/>
            <center>Inspired by FrogFing by ActionRetro, Recreated from scratch by <b>Nihirash</b></center>
            <center>You can <a href="/static/support.html">support this project</a> with donations!</center>
            <center>Powered by SerpAPI, DuckDuckGo and some magic</center>
    }
}
