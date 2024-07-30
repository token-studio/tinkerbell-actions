use std::fmt::Debug;
use std::io::Cursor;

use envy::from_env;
use oci_distribution::client::ImageLayer;
use oci_distribution::{secrets::RegistryAuth, Client, Reference};
use serde::Deserialize;
use tar::Archive;

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

    let reference: Reference = cli.image.parse().expect("Not a valid image reference");
    let auth = if cli.anonymous {
        RegistryAuth::Anonymous
    } else {
        // TODO: auth
        RegistryAuth::Anonymous
    };

    let client_config = build_client_config(&cli);
    let client = Client::new(client_config);
    let accepted_media_types: Vec<&str> = vec!["application/vnd.oci.image.layer.v1.tar"];

    let image = client
        .pull(&reference, &auth, accepted_media_types)
        .await
        .expect("Cannot pull OCI layer")
        .layers
        .into_iter()
        .next()
        .expect("no image found");

    let image_name = get_image_name_from_layer(&image);
    let image_bytes = image.data;

    let mime = new_mime_guess::from_path(image_name)
        .first()
        .expect("mime not found");
    println!("MIME: {}", mime);

    // TODO: decompress
    let decompressed = match mime.to_string().as_str() {
        "application/zstd" => decompress_zstd(&image_bytes),
        "application/x-tar" => image_bytes,
        _ => panic!("Unsupported mime type: {}", mime),
    };

    // TODO: write to disk
    mount_disk(&envs.disk);
    write_to_dir(&decompressed, &envs.disk, &false);
}

fn get_image_name_from_layer(layer: &ImageLayer) -> String {
    layer
        .annotations
        .as_ref()
        .expect("no annotations found")
        .get("org.opencontainers.image.title")
        .expect("no annotation found")
        .to_string()
}

fn decompress_zstd(zstd_bytes: &Vec<u8>) -> Vec<u8> {
    let cursor = Cursor::new(zstd_bytes);
    zstd::decode_all(cursor).expect("Cannot decompress")
}

fn mount_disk(disk: &String) {
    println!("{}", disk);
}

fn write_to_dir(tar_bytes: &Vec<u8>, dest_dir: &String, overwrite: &bool) {
    println!("{}", dest_dir);
    let mut archive = Archive::new(Cursor::new(tar_bytes));
    archive.set_preserve_ownerships(true);
    archive.set_preserve_permissions(true);
    archive.set_ignore_zeros(true);
    archive.set_unpack_xattrs(true);
    archive.set_overwrite(overwrite.to_owned());
    archive.unpack(dest_dir).unwrap();
}
