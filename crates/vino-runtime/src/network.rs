use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;

use vino_lattice::lattice::Lattice;
use vino_transport::TransportMap;
use vino_wascap::KeyPair;

use crate::dev::prelude::*;
use crate::network_service::handlers::initialize::Initialize;
use crate::network_service::handlers::list_schematics::ListSchematics;
pub use crate::providers::network_provider::Provider as NetworkProvider;

type Result<T> = std::result::Result<T, RuntimeError>;
#[derive(Debug)]
#[must_use]
pub struct Network {
  pub uid: String,
  definition: NetworkDefinition,
  addr: Addr<NetworkService>,
  allow_latest: bool,
  allowed_insecure: Vec<String>,
  kp: KeyPair,
  timeout: Duration,
  lattice: Option<Arc<Lattice>>,
}

impl Network {
  pub fn new(definition: NetworkDefinition, seed: &str) -> Result<Self> {
    Ok(NetworkBuilder::from_definition(definition, seed)?.build())
  }

  pub async fn init(&self) -> Result<()> {
    trace!("NETWORK:INIT");
    let kp = KeyPair::new_service();
    let seed = map_err!(kp.seed(), InternalError::E5103)?;
    let init = Initialize {
      network_uid: self.uid.clone(),
      seed,
      lattice: self.lattice.clone(),
      network: self.definition.clone(),
      allowed_insecure: self.allowed_insecure.clone(),
      allow_latest: self.allow_latest,
      timeout: self.timeout,
    };
    map_err!(self.addr.send(init).await, InternalError::E5102)?
      .map_err(|e| RuntimeError::InitializationFailed(e.to_string()))?;
    trace!("NETWORK:INIT:COMPLETE");
    Ok(())
  }

  pub async fn request<T, U>(
    &self,
    schematic: T,
    origin: Entity,
    data: U,
  ) -> Result<TransportStream>
  where
    T: AsRef<str> + Send + Sync,
    U: TryInto<TransportMap> + Send + Sync,
  {
    trace!("NETWORK:REQUEST[{}]", schematic.as_ref());
    let time = std::time::Instant::now();
    let payload = data
      .try_into()
      .map_err(|_| RuntimeError::Serialization("Could not serialize input payload".to_owned()))?;

    let invocation = Invocation::new(
      origin,
      Entity::Schematic(schematic.as_ref().to_owned()),
      payload,
    );

    let response = map_err!(
      self.addr.send(invocation).timeout(self.timeout).await,
      InternalError::E5101
    )?;

    trace!(
      "NETWORK:REQUEST[{}]:COMPLETE[duration {} μs]",
      schematic.as_ref(),
      time.elapsed().as_micros()
    );
    Ok(response.ok()?)
  }

  pub async fn list_schematics(&self) -> Result<Vec<SchematicSignature>> {
    trace!("NETWORK:LIST_SCHEMATICS");
    let msg = ListSchematics {};
    let response = map_err!(
      self.addr.send(msg).timeout(self.timeout).await,
      InternalError::E5102
    )?;
    trace!("NETWORK:LIST_SCHEMATICS:COMPLETE");
    Ok(response?)
  }
}

/// The HostBuilder builds the configuration for a Vino Host.
#[derive(Debug)]
#[must_use]
pub struct NetworkBuilder {
  allow_latest: bool,
  allowed_insecure: Vec<String>,
  definition: NetworkDefinition,
  kp: KeyPair,
  uid: String,
  lattice: Option<Arc<Lattice>>,
  timeout: Duration,
}

impl NetworkBuilder {
  /// Creates a new host builder.
  pub fn from_definition(definition: NetworkDefinition, seed: &str) -> Result<Self> {
    let kp = keypair_from_seed(seed)?;
    let nuid = kp.public_key();
    Ok(Self {
      definition,
      allow_latest: false,
      allowed_insecure: vec![],
      uid: nuid,
      timeout: Duration::from_secs(5),
      lattice: None,
      kp,
    })
  }

  pub fn timeout(self, timeout: Duration) -> Self {
    Self { timeout, ..self }
  }

  pub fn allow_latest(self, allow_latest: bool) -> Self {
    Self {
      allow_latest,
      ..self
    }
  }

  pub fn allow_insecure(self, allowed_insecure: Vec<String>) -> Self {
    Self {
      allowed_insecure,
      ..self
    }
  }

  pub fn lattice(self, lattice: Arc<Lattice>) -> Self {
    Self {
      lattice: Some(lattice),
      ..self
    }
  }

  /// Constructs an instance of a Vino host.
  pub fn build(self) -> Network {
    let addr = crate::network_service::NetworkService::for_id(&self.uid);

    Network {
      addr,
      definition: self.definition,
      uid: self.uid,
      allow_latest: self.allow_latest,
      allowed_insecure: self.allowed_insecure,
      kp: self.kp,
      timeout: self.timeout,
      lattice: self.lattice,
    }
  }
}
