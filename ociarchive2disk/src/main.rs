use std::fmt::Debug;

use envy::from_env;
use oci_distribution::{secrets::RegistryAuth, Client, Reference};
use serde::Deserialize;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Pull a WebAssembly module from a OCI container registry
#[derive(Debug)]
pub(crate) struct Cli {
    /// Perform anonymous operation, by default the tool tries to reuse the docker credentials read
    /// from the default docker file
    pub anonymous: bool,

    /// Pull image from registry using HTTP instead of HTTPS
    pub insecure: bool,

    /// Name of the image to pull
    pub image: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    disk: String,
    url: String,
}

fn build_client_config(cli: &Cli) -> oci_distribution::client::ClientConfig {
    let protocol = if cli.insecure {
        oci_distribution::client::ClientProtocol::Http
    } else {
        oci_distribution::client::ClientProtocol::Https
    };

    oci_distribution::client::ClientConfig {
        protocol,
        ..Default::default()
    }
}

#[tokio::main]
pub async fn main() {
    let envs = match from_env::<Config>() {
        Ok(val) => val,
        Err(error) => {
            panic!("{:#?}", error)
        }
    };
    let cli = Cli {
        insecure: false,
        anonymous: true,
        image: envs.url,
    };
    println!("{:?}", cli);
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();

    let reference: Reference = cli.image.parse().expect("Not a valid image reference");
    let auth: RegistryAuth;
    if cli.anonymous {
        auth = RegistryAuth::Anonymous;
    } else {
        // TODO: auth
        auth = RegistryAuth::Anonymous;
    }

    let client_config = build_client_config(&cli);
    let client = Client::new(client_config);
    let mut accepted_media_types = Vec::new();
    accepted_media_types.push("application/vnd.oci.image.layer.v1.tar");

    let image = client
        .pull(&reference, &auth, accepted_media_types)
        .await
        .expect("Cannot pull OCI layer")
        .layers
        .into_iter()
        .next();

    let binding = image
        .clone()
        .expect("no image found")
        .annotations
        .expect("no annotations found");
    let image_name = binding
        .get("org.opencontainers.image.title")
        .expect("no annotation found");
    let image_bytes = image
        .clone()
        .map(|layer| layer.data)
        .expect("No data found");

    async_std::fs::write(format!("/tmp/{image_name}"), image_bytes)
        .await
        .expect("Cannot write to file");
    let mime = new_mime_guess::from_path(image_name)
        .first()
        .expect("mime not found");
    // TODO: decompress
    if mime == "application/zstd" {}

    // TODO: write to disk
}
