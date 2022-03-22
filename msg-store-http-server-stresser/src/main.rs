use hyper::{Body, Method, Request, Result as HyperResult, Response, Error as HyperError};
use hyper::Client;
use tokio::{self, task::JoinHandle};
use futures::future::*;



use std::fs::{File, create_dir_all, remove_dir_all};
use std::io::{BufReader, BufWriter, Write, Read};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rand::{Rng};
use rayon::prelude::*;
use tempdir::TempDir;

fn get_file_name() -> PathBuf {
    let characters = "qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890".as_bytes().to_vec();
    let mut rng = rand::thread_rng();
    let mut name_parts: Vec<u8> = Vec::with_capacity(30);
    for _ in 0..30 {
        let new_char = characters[rng.gen_range(0..characters.len())];
        name_parts.push(new_char);
    }
    let name = String::from_utf8(name_parts).unwrap();
    PathBuf::from(name)    
}

fn generate_contents(iterations: u32, writer: &mut BufWriter<File>) {
    let characters = "qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890!@#$%^&*()~`[]{};?><,./|".as_bytes().to_vec();
    let mut rng = rand::thread_rng();
    for _ in 0..iterations {
        let mut chunk = Vec::with_capacity(1024);
        for _ in 0..1024 {
            chunk.push(characters[rng.gen_range(0..characters.len())]);
        }
        writer.write(&chunk).unwrap();
    }
}

fn create_file(source_path: &Path) -> PathBuf {
    let file_name = get_file_name();
    let file_path = {
        let mut file_path = source_path.to_path_buf();
        file_path.push(&file_name);
        file_path
    };
    // let file_path = format!("{}/{}", source_path, file_name);
    let file = File::create(file_path).unwrap();
    let mut buf = BufWriter::new(file);
    generate_contents(10000, &mut buf);
    file_name
}

fn create_files(source_path: &Path, count: u32) -> Vec<PathBuf> {
    let next = Arc::new(Mutex::new(0));
    (1..=count).into_par_iter().map(|_| {
        {
            let mut counter = next.lock().unwrap();
            if *counter % 10 == 0 {
                println!("Creating file {}", counter);
            }
            *counter += 1;
        }
        create_file(&source_path)
    }).collect::<Vec<PathBuf>>()
}

async fn send_file(source_dir: &Path, file_name: PathBuf) {
    let file_path = {
        let mut file_path = source_dir.to_path_buf();
        file_path.push(&file_name);
        file_path
    };
    let mut file = File::open(file_path).unwrap();
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    let client = Client::new();
    let req = Request::builder()
        .method(Method::POST)
        .uri("http://127.0.0.1:8080/api/msg")
        .body(Body::from(format!("priority=1&saveToFile=true&bytesizeOverride=1&fileName={}?{}", file_name.to_str().unwrap(), buf)))
        .expect("request builder");
    client.request(req).await.unwrap();
    // println!("{:#?}", client.request(req).await.unwrap().into_body());
}

async fn send_files(source_path: PathBuf, files: Vec<PathBuf>) {
    let mut index = 0;
    let mut finished = false;
    loop {
        if finished {
            break;
        }
        let mut handles: Vec<JoinHandle<Result<(), ()>>> = Vec::with_capacity(10);
        println!("Sending file {}", index);
        for _ in 0..9 {
            if index == files.len() - 1 {
                finished = true;
                break
            }
            let source_path_copy = source_path.to_path_buf();
            let file_copy = files[index].to_path_buf();
            handles.push(tokio::spawn(async move {
                send_file(&source_path_copy, file_copy).await;
                Ok(())
            }));
            index += 1;
        }
        futures::future::join_all(handles).await;
    }    
}

async fn send_message() {
    let client = Client::new();
    let req = Request::builder()
        .method(Method::POST)
        .uri("http://127.0.0.1:8080/api/msg")
        .body(Body::from("priority=1?hello, world!"))
        .expect("request builder");
    client.request(req).await.unwrap();
    // println!("{:#?}", client.request(req).await.unwrap().into_body());
}

async fn start_server(msg_store_dir: PathBuf) -> JoinHandle<Output> {
    tokio::spawn(async move {
        Command::new("msg-store-http-server").args([
            "--node-id=12345",
            "--database=leveldb",
            &format!("--leveldb-path={}/leveldb", msg_store_dir.to_str().unwrap()),
            &format!("--file-storage-path={}/file-storage", msg_store_dir.to_str().unwrap())
        ]).output().unwrap()
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let msg_store_dir = Path::new("msg-store-test");
    let source_path = {
        let mut source_path = msg_store_dir.to_path_buf();
        source_path.push("source-files");
        source_path
    };
    create_dir_all(&source_path).unwrap();
    println!("Creating files...");
    let file_names = create_files(&source_path, 1000);
    println!("Sending files...");
    send_files(source_path, file_names).await;
    println!("Removing tmp dir...");
    remove_dir_all(msg_store_dir).unwrap();
    Ok(())
}