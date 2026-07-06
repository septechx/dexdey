use futures::StreamExt;
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt};

#[tokio::main]
async fn main() {
    let out = PathBuf::from("java_libs");
    fs::create_dir_all(&out).await.unwrap();

    let packages = [
        "maven$org/slf4j/slf4j-api:2.0.18",
        "maven$org/slf4j/slf4j-simple:2.0.18",
        "paper$com/velocitypowered/velocity-api:3.5.0-SNAPSHOT",
    ];

    let tasks = packages.into_iter().map(|package| download(&out, package));

    futures::future::join_all(tasks).await;

    println!("cargo:rerun-if-changed=build.rs");
}

async fn download(out: &Path, package: &str) {
    let (repo, package) = package.split_once('$').unwrap();
    let (name, version) = package.split_once(':').unwrap();
    let (_, artifact) = name.rsplit_once('/').unwrap();

    if out.join(format!("{artifact}.jar")).exists() {
        return;
    }

    let repo = get_repo(repo);

    println!("cargo:warning=Downloading {package} from {repo}");

    let base_url = format!("https://{repo}/{name}/{version}");
    let resolved_version = if version.ends_with("-SNAPSHOT") {
        resolve_snapshot(&base_url).await
    } else {
        version.to_string()
    };

    let url = format!("{base_url}/{artifact}-{resolved_version}.jar");

    let response = reqwest::get(&url).await.unwrap();
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap();
        panic!("Failed to download {url}: {status} - {text}");
    }

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

async fn resolve_snapshot(base_url: &str) -> String {
    let metadata_url = format!("{base_url}/maven-metadata.xml");
    let resp = reqwest::get(&metadata_url).await.unwrap();
    let text = resp.text().await.unwrap();

    let doc = roxmltree::Document::parse(&text).unwrap();

    let value = doc
        .descendants()
        .filter(|n| n.has_tag_name("snapshotVersion"))
        .find_map(|sv| {
            let ext = sv
                .descendants()
                .find(|n| n.has_tag_name("extension"))?
                .text()?;
            if ext == "jar" {
                sv.descendants().find(|n| n.has_tag_name("value"))?.text()
            } else {
                None
            }
        })
        .unwrap();

    value.to_string()
}

fn get_repo(repo: &str) -> &str {
    match repo {
        "maven" => "repo1.maven.org/maven2",
        "paper" => "repo.papermc.io/repository/maven-public",
        _ => panic!("Unknown repo: {}", repo),
    }
}
