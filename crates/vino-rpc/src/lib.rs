//! Vino RPC implementation

#![deny(
  warnings,
  missing_debug_implementations,
  trivial_casts,
  trivial_numeric_casts,
  unsafe_code,
  unstable_features,
  unused_import_braces,
  unused_qualifications,
  type_alias_bounds,
  trivial_bounds,
  mutable_transmutes,
  invalid_value,
  explicit_outlives_requirements,
  deprecated,
  clashing_extern_declarations,
  clippy::expect_used,
  clippy::explicit_deref_methods,
  // missing_docs
)]
#![warn(clippy::cognitive_complexity)]

pub mod error;
pub mod generated;
pub mod invocation_server;
pub mod port;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
pub use generated::vino as rpc;
use generated::vino::component::ComponentKind;
use generated::vino::invocation_service_client::InvocationServiceClient;
use generated::vino::invocation_service_server::InvocationServiceServer;
pub use invocation_server::InvocationServer;
use port::PortPacket;
use serde::{
  Deserialize,
  Serialize,
};
use tokio::task::JoinHandle;
use tonic::transport::{
  Channel,
  Server,
  Uri,
};
use vino_component::v0::Payload;
use vino_component::Packet;

use crate::rpc::OutputSignal;

pub type Result<T> = std::result::Result<T, error::RpcError>;
pub type Error = crate::error::RpcError;
pub type RpcResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate derivative;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ComponentSignature {
  pub name: String,
  pub inputs: Vec<PortSignature>,
  pub outputs: Vec<PortSignature>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PortSignature {
  pub name: String,
  pub type_string: String,
}

impl PortSignature {
  pub fn new(name: String, type_string: String) -> Self {
    Self { name, type_string }
  }
}

impl From<(String, String)> for PortSignature {
  fn from(tup: (String, String)) -> Self {
    let (name, type_string) = tup;
    Self { name, type_string }
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderSignature {
  pub name: String,
  pub components: Vec<ComponentSignature>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SchematicSignature {
  pub name: String,
  pub inputs: Vec<PortSignature>,
  pub outputs: Vec<PortSignature>,
  pub providers: Vec<ProviderSignature>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum HostedType {
  Component(ComponentSignature),
  Schematic(SchematicSignature),
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Statistics {
  pub num_calls: u64,
  pub execution_duration: ExecutionStatistics,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ExecutionStatistics {
  pub max_time: usize,
  pub min_time: usize,
  pub average: usize,
}

pub type BoxedPacketStream = Pin<Box<dyn Stream<Item = PortPacket> + Send>>;

#[async_trait]
pub trait RpcHandler: Send + Sync {
  async fn request(
    &self,
    inv_id: String,
    component: String,
    payload: HashMap<String, Vec<u8>>,
  ) -> RpcResult<BoxedPacketStream>;
  async fn list_registered(&self) -> RpcResult<Vec<HostedType>>;
  async fn report_statistics(&self, id: Option<String>) -> RpcResult<Vec<Statistics>>;
}

impl From<HostedType> for crate::rpc::Component {
  fn from(v: HostedType) -> Self {
    match v {
      HostedType::Component(v) => v.into(),
      HostedType::Schematic(v) => v.into(),
    }
  }
}

impl TryFrom<crate::rpc::Component> for HostedType {
  type Error = Error;

  fn try_from(value: crate::rpc::Component) -> Result<Self> {
    let kind = ComponentKind::from_i32(value.kind)
      .ok_or_else(|| Error::Other("Invalid component kind".to_string()))?;

    match kind {
      ComponentKind::Component => Ok(HostedType::Component(ComponentSignature {
        name: value.name,
        inputs: value.inputs.into_iter().map(From::from).collect(),
        outputs: value.outputs.into_iter().map(From::from).collect(),
      })),
      ComponentKind::Schematic => Ok(HostedType::Schematic(SchematicSignature {
        name: value.name,
        inputs: value.inputs.into_iter().map(From::from).collect(),
        outputs: value.outputs.into_iter().map(From::from).collect(),
        providers: value.providers.into_iter().map(From::from).collect(),
      })),
    }
  }
}

impl From<crate::generated::vino::Provider> for ProviderSignature {
  fn from(v: crate::generated::vino::Provider) -> Self {
    Self {
      name: v.name,
      components: v.components.into_iter().map(From::from).collect(),
    }
  }
}

impl From<crate::generated::vino::Component> for ComponentSignature {
  fn from(v: crate::generated::vino::Component) -> Self {
    Self {
      name: v.name,
      inputs: v.inputs.into_iter().map(From::from).collect(),
      outputs: v.outputs.into_iter().map(From::from).collect(),
    }
  }
}

impl From<ComponentSignature> for crate::generated::vino::Component {
  fn from(v: ComponentSignature) -> Self {
    Self {
      name: v.name,
      kind: crate::rpc::component::ComponentKind::Component.into(),
      inputs: v.inputs.into_iter().map(From::from).collect(),
      outputs: v.outputs.into_iter().map(From::from).collect(),
      providers: vec![],
    }
  }
}

impl From<SchematicSignature> for crate::generated::vino::Component {
  fn from(v: SchematicSignature) -> Self {
    Self {
      name: v.name,
      kind: crate::rpc::component::ComponentKind::Schematic.into(),
      inputs: v.inputs.into_iter().map(From::from).collect(),
      outputs: v.outputs.into_iter().map(From::from).collect(),
      providers: v.providers.into_iter().map(From::from).collect(),
    }
  }
}

impl From<ProviderSignature> for crate::generated::vino::Provider {
  fn from(v: ProviderSignature) -> Self {
    Self {
      name: v.name,
      components: v.components.into_iter().map(From::from).collect(),
    }
  }
}

impl From<PortSignature> for crate::generated::vino::component::Port {
  fn from(v: PortSignature) -> Self {
    Self {
      name: v.name,
      r#type: v.type_string,
    }
  }
}

impl From<crate::generated::vino::component::Port> for PortSignature {
  fn from(v: crate::generated::vino::component::Port) -> Self {
    Self {
      name: v.name,
      type_string: v.r#type,
    }
  }
}

impl From<Statistics> for crate::generated::vino::Statistic {
  fn from(v: Statistics) -> Self {
    Self {
      num_calls: v.num_calls,
    }
  }
}

impl From<crate::generated::vino::Statistic> for Statistics {
  fn from(v: crate::generated::vino::Statistic) -> Self {
    Self {
      num_calls: v.num_calls,
      execution_duration: ExecutionStatistics::default(),
    }
  }
}

#[allow(clippy::from_over_into)]
impl Into<Packet> for rpc::OutputKind {
  fn into(self) -> Packet {
    use rpc::output_kind::Data;
    match self.data {
      Some(v) => match v {
        Data::Messagepack(v) => Packet::V0(Payload::MessagePack(v)),
        Data::Error(v) => Packet::V0(Payload::Error(v)),
        Data::Exception(v) => Packet::V0(Payload::Exception(v)),
        Data::Test(_) => Packet::V0(Payload::Invalid),
        Data::Invalid(_) => Packet::V0(Payload::Invalid),
        Data::Signal(signal) => match OutputSignal::from_i32(signal) {
          Some(OutputSignal::Close) => Packet::V0(Payload::Close),
          Some(OutputSignal::OpenBracket) => Packet::V0(Payload::OpenBracket),
          Some(OutputSignal::CloseBracket) => Packet::V0(Payload::CloseBracket),
          None => Packet::V0(Payload::Error("Sent an invalid signal".to_string())),
        },
      },
      None => Packet::V0(Payload::Error("Response received without output".into())),
    }
  }
}

/// Build and spawn an RPC server for the passed provider
pub fn make_rpc_server(
  socket: tokio::net::TcpSocket,
  provider: impl RpcHandler + 'static,
) -> JoinHandle<std::result::Result<(), tonic::transport::Error>> {
  let component_service = InvocationServer::new(provider);

  let svc = InvocationServiceServer::new(component_service);

  let listener = tokio_stream::wrappers::TcpListenerStream::new(socket.listen(1).unwrap());

  tokio::spawn(
    Server::builder()
      .add_service(svc)
      .serve_with_incoming(listener),
  )
}

/// Create an RPC client
pub async fn make_rpc_client(uri: Uri) -> Result<InvocationServiceClient<Channel>> {
  Ok(InvocationServiceClient::connect(uri).await?)
}

pub fn bind_new_socket() -> Result<tokio::net::TcpSocket> {
  let socket = tokio::net::TcpSocket::new_v4()?;
  socket.bind("127.0.0.1:0".parse().unwrap())?;
  Ok(socket)
}
