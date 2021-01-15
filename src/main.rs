use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};

use std::borrow::Borrow;
use url::{ParseError, Url};

use async_std::task;
use surf;



type CrawlerResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

type BoxFuture = std::pin::Pin<Box<dyn std::future::Future<Output = CrawlerResult> + Send>>;

#[derive(Debug, Default)]
struct LinkQueue {
    links: Vec<String>,
}

impl TokenSink for &mut LinkQueue {

    type Handle = ();

    fn process_token(&mut self, token: Token, line_number: u64) -> TokenSinkResult<Self::Handle> {
        match token {
            TagToken(
                ref tag @ Tag {
                    kind: TagKind::StartTag,
                    ..
                },
            ) => {

                if tag.name.as_ref() == "a" {
                    for attributes in tag.attrs.iter(){
                        if attributes.nme.local.as_ref() == "href" {
                            let url_str: &[u8] = attributes.value.borrow();
                            self.links.push(String::from_utf8_lossy(url_str).into_owned());
                        }

                    }
                }
            
        }
        _ => {}
        }
        TokenSinkResult::Continue
    }

}

pub fn get_links(url: &Url, page: String) -> Vec<Url> {
    let mut domain_url = url.clone();
    let mut queue = LinkQueue::default(); 
    let mut tokenizer = Tokenizer::new(&mut queue, TokenizerOpts::default());
    let mut buffer = BufferQueue::new();
    buffer.push_back(page.into());
    let _ = tokenizer.feed(&mut buffer);

    queue.links.iter().map(|link| match Url::parse(link) {
        Ok(url) => url,
        Err(ParseError::RelativeUrlWithoutBase) => domain_url.join(link).unwrap(),
        Err(_)=> panic!("Bad link found: {:?}", link)
    }).collect()
}

fn box_crawl(pages: vec<Url>, current: u8, max: u8)->BoxFuture{
    Box::pin(crawl(pages, current, max))
}


async fn crawlpages(pages: Vec<Url>, current: u8, max: u8) -> CrawlerResult {

    println!("current depth {}, max depth {}", current, max);

    if current > max {
        println!("reached max depth");
        return Ok(());
    }

    let mut tasks = vec![];
    println!("pages {:?}", pages);

    for url in pages{
        let task = task::spawn(async move{
            println!(" getting {}", url);
            let mut res = surf::get(&url).await?;
            let body = res.body_string().await?;
            let links = get_links(&url, body).await;
            println!("Following: {:?}", links);
            box_crawl(links, current + 1, max).await;
        });
        tasks.push(task);
    }

    for task in tasks.into_iter(){
        task.await;
    }
    Ok(())

}
fn main() {

    task::block_on(async{
        box_crawl(vec![Url::parse("https://www.rust-lang.org").unwrap()], 1 , 2).await;
    })
}
