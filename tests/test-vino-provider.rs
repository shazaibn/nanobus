use futures::prelude::*;
use log::debug;
use maplit::hashmap;
use test_vino_provider::Provider;
use vino_codec::messagepack::{
  deserialize,
  serialize,
};
use vino_component::{
  v0,
  Packet,
};
use vino_rpc::RpcHandler;

#[test_env_log::test(tokio::test)]
async fn request() -> anyhow::Result<()> {
  let provider = Provider::default();
  let input = "some_input";
  let invocation_id = "INVOCATION_ID";
  let job_payload = hashmap! {
    "input".to_string() => serialize(input)?,
  };

  let mut outputs = provider
    .request(
      invocation_id.to_string(),
      vino_entity::Entity::component("test-component"),
      job_payload,
    )
    .await
    .expect("request failed");
  let output = outputs.next().await.unwrap();
  println!("Received payload from [{}]", output.port);
  let payload: String = match output.packet {
    Packet::V0(v0::Payload::MessagePack(payload)) => deserialize(&payload)?,
    _ => None,
  }
  .unwrap();

  println!("outputs: {:?}", payload);
  assert_eq!(payload, "TEST: some_input");

  Ok(())
}

#[test_env_log::test(tokio::test)]
async fn list() -> anyhow::Result<()> {
  let provider = Provider::default();

  let response = provider.list_registered().await.expect("request failed");
  debug!("list response : {:?}", response);

  Ok(())
}
