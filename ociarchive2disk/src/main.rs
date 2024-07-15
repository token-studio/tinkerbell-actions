use std::fmt::Debug;

use envy::from_env;
use oci_distribution::{Client, Reference, secrets::RegistryAuth};
use serde::Deserialize;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Pull a WebAssembly module from a OCI container registry
#[derive(Debug)]
pub(crate) struct Cli {
    /// Enable verbose mode
    pub verbose: bool,

    /// Perform anonymous operation, by default the tool tries to reuse the docker credentials read
    /// from the default docker file
    pub anonymous: bool,

    /// Pull image from registry using HTTP instead of HTTPS
    pub insecure: bool,

    /// Enable json output
    pub json: bool,

    /// Name of the image to pull
    image: String,
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
        verbose: true,
        insecure: false,
        json: true,
        anonymous: true,
        image: envs.url,
    };
    println!("{:?}", cli);
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();

    let reference: Reference = cli.image.parse().expect("Not a valid image reference");
    let auth = RegistryAuth::Anonymous;

    let client_config = build_client_config(&cli);
    let client = Client::new(client_config);
    let mut accepted_media_types = Vec::new();
    accepted_media_types.push("application/vnd.oci.image.layer.v1.tar");

    let image = client
        .pull(&reference, &auth, accepted_media_types)
        .await
        .expect("Cannot pull Wasm module")
        .layers
        .into_iter()
        .next()
        .map(|layer| layer.data)
        .expect("No data found");
    async_std::fs::write(envs.disk, image)
        .await
        .expect("Cannot write to file");
    // TODO: decompress
    // TODO: write to disk
}
