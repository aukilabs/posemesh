use anyhow::{bail, Context, Result};
use auki_p2p::Identity;
use std::{
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

fn main() -> Result<()> {
    let mut args = env::args_os();
    let program = args
        .next()
        .unwrap_or_else(|| OsString::from("posemesh-p2p-keygen"));
    let Some(destination) = args.next() else {
        bail!("usage: {} OUTPUT_FILE", Path::new(&program).display());
    };
    if args.next().is_some() {
        bail!("usage: {} OUTPUT_FILE", Path::new(&program).display());
    }

    let destination = PathBuf::from(destination);
    let peer_id = generate_key_file(&destination)?;
    println!("created {}", destination.display());
    println!("peer_id={peer_id}");
    Ok(())
}

fn generate_key_file(destination: &Path) -> Result<auki_p2p::PeerId> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let file_name = destination
        .file_name()
        .filter(|name| !name.is_empty())
        .context("OUTPUT_FILE must name a file")?;
    if !parent.is_dir() {
        bail!("OUTPUT_FILE parent directory does not exist");
    }
    if destination.exists() {
        bail!("refusing to overwrite existing OUTPUT_FILE");
    }

    let identity = Identity::generate();
    let encoded = identity
        .to_protobuf_encoding()
        .context("encode generated libp2p private key")?;

    let mut last_collision = None;
    for _ in 0..16 {
        let temporary = temporary_path(parent, file_name);
        match write_private_file(&temporary, &encoded) {
            Ok(()) => {
                let link_result = fs::hard_link(&temporary, destination);
                let cleanup_result = fs::remove_file(&temporary);
                match link_result {
                    Ok(()) => {
                        cleanup_result.with_context(|| {
                            format!("remove temporary key file {}", temporary.display())
                        })?;
                        return Ok(identity.peer_id());
                    }
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                        let _ = cleanup_result;
                        bail!("refusing to overwrite existing OUTPUT_FILE");
                    }
                    Err(error) => {
                        let _ = cleanup_result;
                        return Err(error).with_context(|| {
                            format!(
                                "atomically install generated private key at {}",
                                destination.display()
                            )
                        });
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                last_collision = Some(error);
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("create temporary key file beside {}", destination.display())
                });
            }
        }
    }

    Err(last_collision.unwrap_or_else(|| io::Error::from(io::ErrorKind::AlreadyExists)))
        .context("could not allocate a unique temporary key file")
}

fn temporary_path(parent: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let suffix = rand::random::<u64>();
    let mut temporary_name = OsString::from(".");
    temporary_name.push(file_name);
    temporary_name.push(format!(".tmp-{}-{suffix:016x}", std::process::id()));
    parent.join(temporary_name)
}

fn write_private_file(path: &Path, encoded: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    if let Err(error) = file.write_all(encoded).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_file_round_trips_and_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("libp2p-private-key");

        let peer_id = generate_key_file(&path).unwrap();
        let encoded = fs::read(&path).unwrap();
        assert_eq!(
            Identity::from_protobuf_encoding(&encoded)
                .unwrap()
                .peer_id(),
            peer_id
        );

        let original = encoded;
        assert!(generate_key_file(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
