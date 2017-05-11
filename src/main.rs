extern crate pink_spider;
extern crate hyper;
extern crate html5ever;
extern crate mecab;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use html5ever::tendril::stream::TendrilSink;
use pink_spider::model::{Model, Entry};
use pink_spider::http;
use pink_spider::error::Error;
use hyper::header::Connection;
use hyper::header::ConnectionOption;
use html5ever::rcdom::{RcDom, Handle};
use html5ever::{parse_document};
use html5ever::rcdom::NodeData::{
    Document,
    Doctype,
    Text,
    Comment,
    Element,
    ProcessingInstruction
};
use mecab::Tagger;

pub fn main() {
    let mut page = 0;
    let per_page = 10;
    let mut entries = Entry::find(page, 0);
    let total = entries.total;
    let mut index = 0;
    println!("total_count {}", total);
    while index < total {
        entries = Entry::find(page, per_page);
        for entry in entries.items.iter() {
            println!("processing {}", index);
            process_entry(entry);
            index += 1;
        };
        let len = entries.items.len() as i64;
        index = page * per_page + len;
        page += 1;
    }
}

trait Asset {
    fn asset_file_name(&self) -> String;
}

impl Asset for Entry {
    fn asset_file_name(&self) -> String {
        let mut file_name = self.url.to_string();
        file_name = file_name.replace("http://" , "");
        file_name = file_name.replace("https://", "");
        file_name = file_name.replace("/", "_");
        file_name = format!("assets/{}.txt", file_name).to_owned();
        file_name
    }
}

pub fn process_entry(entry: &Entry) {
    let asset_file_name = entry.asset_file_name();
    let asset_file_path = Path::new(&asset_file_name);
    if asset_file_path.exists() {
        println!("already exists");
        return;
    }
    let mut tokenized = String::new();
    match extract(&entry.url) {
        Ok(content) => {
            println!("creating {:?} ...", asset_file_path);
            tokenize(&content, &mut tokenized);
            match File::create(asset_file_path.clone()) {
                Ok(mut f) => f.write_all(tokenized.as_bytes()).unwrap(),
                Err(e)    => println!("{}", e),
            }
            println!("created {:?}", asset_file_path);
        },
        Err(_) => (),
    }
}

pub fn tokenize(input: &str, output: &mut String) {
    let mut tagger = Tagger::new("");
    for node in tagger.parse_to_node(input).iter_next() {
        match node.stat as i32 {
            mecab::MECAB_BOS_NODE => {},
            mecab::MECAB_EOS_NODE => {},
            _ => {
                output.push_str(" ");
                output.push_str(&(node.surface)[..(node.length as usize)]);
            },
        }
    }
}

pub fn extract(url: &str) -> Result<String, Error> {
    let client  = http::client();
    let builder = client.get(url)
        .header(Connection(vec![ConnectionOption::Close]));
    let mut res = try!(builder.send());
    if res.status.is_success() {
        let dom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut res)
            .unwrap();
        let mut content   = String::new();
        walk(dom.document, None, &mut content);
        Ok(content)
    } else {
        Err(Error::NotFound)
    }
}

fn walk(handle:    Handle,
        tag:       Option<&str>,
        content:   &mut String) {
    let mut tag = tag;
    match handle.data {
        Document       => (),
        Doctype { .. } => (),
        Text { ref contents } => {
            match tag {
                Some("script") => (),
                Some("SCRIPT") => (),
                Some("style")  => (),
                Some("STYLE")  => (),
                _              => content.push_str(&contents.borrow()),
            }
        },
        Comment { .. } => (),
        Element { ref name, .. } => {
            let tag_name = name.local.as_ref();
            tag = Some(tag_name);
        },
        ProcessingInstruction { .. } => unreachable!()
    }
    for child in handle.children.borrow().iter() {
        walk(child.clone(), tag, content);
    }
}
