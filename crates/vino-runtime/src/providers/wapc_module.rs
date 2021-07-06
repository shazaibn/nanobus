use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use vino_wascap::{
  Claims,
  ComponentClaims,
  Token,
};

use super::wapc_provider_service::WapcProviderService;
use crate::dev::prelude::*;
use crate::error::ComponentError;
use crate::utils::oci::fetch_oci_bytes;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct WapcModule {
  pub(crate) token: Token<ComponentClaims>,
  pub(crate) bytes: Vec<u8>,
  #[derivative(Debug = "ignore")]
  pub(crate) addr: Option<Addr<WapcProviderService>>,
}

impl WapcModule {
  /// Create an actor from the bytes of a signed WebAssembly module. Attempting to load
  /// an unsigned module, or a module signed improperly, will result in an error.
  pub fn from_slice(buf: &[u8]) -> Result<WapcModule, ComponentError> {
    let token = vino_wascap::extract_claims(&buf)?;
    token.map_or(
      Err(ComponentError::ClaimsError(
        "Could not extract claims from component".to_owned(),
      )),
      |t| {
        Ok(WapcModule {
          token: t,
          bytes: buf.to_vec(),
          addr: None,
        })
      },
    )
  }

  /// Create an actor from a signed WebAssembly (`.wasm`) file.
  pub fn from_file(path: &Path) -> Result<WapcModule, CommonError> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    WapcModule::from_slice(&buf).map_err(|_| CommonError::FileNotFound(path.to_path_buf()))
  }

  /// Obtain the issuer's public key as it resides in the actor's token (the `iss` field of the JWT).
  pub fn _issuer(&self) -> String {
    self.token.claims.issuer.clone()
  }

  /// Obtain the list of tags in the actor's token.
  pub fn _tags(&self) -> Vec<String> {
    match self.token.claims.metadata.as_ref().unwrap().tags {
      Some(ref tags) => tags.clone(),
      None => vec![],
    }
  }

  /// Obtain the actor's public key (The `sub` field of the JWT).
  pub fn public_key(&self) -> String {
    self.token.claims.subject.clone()
  }

  /// A globally referencable ID to this component
  pub fn id(&self) -> String {
    self.public_key()
  }

  /// The actor's human-friendly display name
  pub fn name(&self) -> String {
    self
      .token
      .claims
      .metadata
      .as_ref()
      .unwrap()
      .interface
      .name
      .clone()
  }

  // Obtain the raw set of claims for this actor.
  pub fn claims(&self) -> Claims<ComponentClaims> {
    self.token.claims.clone()
  }
}

async fn start_wapc_actor_from_file(p: &Path) -> Result<WapcModule, ComponentError> {
  let component = WapcModule::from_file(p)?;
  trace!(
    "Starting wapc component '{}' from file {}",
    component.name(),
    p.to_string_lossy()
  );
  Ok(component)
}

async fn start_wapc_actor_from_oci(
  url: &str,
  allow_latest: bool,
  allowed_insecure: &[String],
) -> Result<WapcModule, ComponentError> {
  let bytes = fetch_oci_bytes(url, allow_latest, allowed_insecure).await?;
  let component = WapcModule::from_slice(&bytes)?;

  trace!(
    "Starting wapc component '{}' from URL {}",
    component.name(),
    url
  );

  Ok(component)
}

pub(crate) async fn load_component(
  comp_ref: String,
  allow_latest: bool,
  allowed_insecure: &[String],
) -> Result<WapcModule, ComponentError> {
  let p = Path::new(&comp_ref);
  let component = if p.exists() {
    debug!("{:?} exists on file system, loading from disk", p);
    start_wapc_actor_from_file(p).await
  } else {
    debug!(
      "{:?} does not exist on file system, trying as OCI url",
      comp_ref
    );
    start_wapc_actor_from_oci(&comp_ref, allow_latest, allowed_insecure).await
  };
  component
}
