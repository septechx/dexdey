use futures::StreamExt;
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt};

#[tokio::main]
async fn main() {
    let out = PathBuf::from("java_libs");
    fs::create_dir_all(&out).await.unwrap();

    let packages = [
        "org/slf4j/slf4j-api:2.0.18",
        "org/slf4j/slf4j-simple:2.0.18",
    ];

    let tasks = packages.into_iter().map(|package| download(&out, package));

    futures::future::join_all(tasks).await;

    println!("cargo:rerun-if-changed=build.rs");
}

async fn download(out: &Path, package: &str) {
    let (name, version) = package.split_once(':').unwrap();
    let (_, artifact) = name.rsplit_once('/').unwrap();

    if out.join(format!("{artifact}.jar")).exists() {
        return;
    }

    eprintln!("Downloading {}", package);

    let url = format!("https://repo1.maven.org/maven2/{name}/{version}/{artifact}-{version}.jar");

    let response = reqwest::get(url).await.unwrap();

    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(out.join(format!("{artifact}.jar")))
        .await
        .unwrap();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        file.write_all(&chunk).await.unwrap();
    }

    file.flush().await.unwrap();
}
