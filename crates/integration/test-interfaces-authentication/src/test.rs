use std::collections::HashMap;

use anyhow::Result;
use maplit::hashmap;
use tonic::transport::Channel;
use tracing::*;
use vino_codec::messagepack::serialize;
use vino_component::{
  v0,
  Packet,
};
use vino_provider::entity::Entity;
use vino_rpc::rpc::invocation_service_client::InvocationServiceClient;
use vino_rpc::rpc::{
  Invocation,
  ListRequest,
};
use vino_rpc::RpcHandler;

async fn list_components(port: &u16) -> Result<Vec<vino_rpc::rpc::Component>> {
  let mut client = InvocationServiceClient::connect(format!("http://127.0.0.1:{}", port)).await?;
  let request = ListRequest {};
  let response = client.list(request).await?.into_inner();

  println!("Output = {:?}", response);
  Ok(response.components)
}

fn make_invocation(target: &str, payload: HashMap<String, Vec<u8>>) -> Result<Invocation> {
  Ok(Invocation {
    origin: Entity::test("test").url(),
    target: Entity::component(target).url(),
    msg: payload,
    id: "".to_string(),
    network_id: "".to_string(),
  })
}

async fn create_user(
  client: &mut InvocationServiceClient<Channel>,
  username: &str,
  user_id: &str,
  password: &str,
) -> Result<String> {
  let payload = hashmap! {
    "user_id".to_string() => serialize(user_id)?,
    "username".to_string() => serialize(username)?,
    "password".to_string()=> serialize(password)?,
  };
  let invocation = make_invocation("create-user", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "user_id");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn remove_user(
  client: &mut InvocationServiceClient<Channel>,
  username: &str,
) -> Result<String> {
  let payload = hashmap! {
    "username".to_string() => serialize(username)?,
  };
  let invocation = make_invocation("remove-user", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "user_id");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn list_users(
  client: &mut InvocationServiceClient<Channel>,
  offset: i32,
  limit: i32,
) -> Result<HashMap<String, String>> {
  let payload = hashmap! {
    "limit".to_string() => serialize(limit)?,
    "offset".to_string() => serialize(offset)?,
  };
  let invocation = make_invocation("list-users", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "users");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn authenticate(
  client: &mut InvocationServiceClient<Channel>,
  username: &str,
  password: &str,
  session: &str,
) -> Result<String> {
  let payload = hashmap! {
    "username".to_string()=> serialize(username)?,
    "password".to_string()=> serialize(password)?,
    "session".to_string()=> serialize(session)?,
  };

  let invocation = make_invocation("authenticate", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "session");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn validate_session(
  client: &mut InvocationServiceClient<Channel>,
  session: &str,
) -> Result<String> {
  let payload = hashmap! {
    "session".to_string()=> serialize(session)?,
  };

  let invocation = make_invocation("validate-session", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "user_id");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn update_permissions(
  client: &mut InvocationServiceClient<Channel>,
  user_id: &str,
  permissions: &[&str],
) -> Result<Vec<String>> {
  let payload = hashmap! {
    "user_id".to_string()=> serialize(user_id)?,
    "permissions".to_string()=> serialize(permissions)?,
  };

  let invocation = make_invocation("update-permissions", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "permissions");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn list_permissions(
  client: &mut InvocationServiceClient<Channel>,
  user_id: &str,
) -> Result<Vec<String>> {
  let payload = hashmap! {
    "user_id".to_string()=> serialize(user_id)?,
  };

  let invocation = make_invocation("list-permissions", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "permissions");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn has_permission(
  client: &mut InvocationServiceClient<Channel>,
  user_id: &str,
  permission: &str,
) -> Result<Packet> {
  let payload = hashmap! {
    "user_id".to_string()=> serialize(user_id)?,
    "permission".to_string()=> serialize(permission)?,
  };

  let invocation = make_invocation("has-permission", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "user_id");
  let next: Packet = next.payload.unwrap().into();
  Ok(next)
}

async fn get_id(client: &mut InvocationServiceClient<Channel>, username: &str) -> Result<String> {
  let payload = hashmap! {
    "username".to_string()=> serialize(username)?,
  };

  let invocation = make_invocation("get-id", payload)?;
  let mut stream = client.invoke(invocation).await?.into_inner();

  let next = stream.message().await?.unwrap();
  println!("Output = {:?}", next);
  assert_eq!(next.port, "user_id");
  let next: Packet = next.payload.unwrap().into();
  Ok(next.try_into()?)
}

async fn test_create_user(port: &u16) -> Result<()> {
  let mut client = InvocationServiceClient::connect(format!("http://127.0.0.1:{}", port)).await?;
  info!("Connected to server");
  let username = "jane@domain.com";
  let user_id = "someid";
  let password = "password123";
  let user_id2 = create_user(&mut client, username, user_id, password).await?;
  assert_eq!(user_id, user_id2);
  let user_id2 = get_id(&mut client, username).await?;
  assert_eq!(user_id, user_id2);
  let user_id2 = remove_user(&mut client, username).await?;
  assert_eq!(user_id, user_id2);
  Ok(())
}

async fn test_list_users(port: &u16) -> Result<()> {
  let mut client = InvocationServiceClient::connect(format!("http://127.0.0.1:{}", port)).await?;
  info!("Connected to server");
  let username = "jane@domain.com";
  let user_id = "someid";
  let password = "password123";
  let users = list_users(&mut client, 0, 100).await?;
  debug!("List users: {:?}", users);
  let num_users = users.len();
  let user_id2 = create_user(&mut client, username, user_id, password).await?;
  debug!("User id: {}", user_id2);
  let users = list_users(&mut client, 0, 100).await?;
  debug!("List users: {:?}", users);
  assert_eq!(users.len(), num_users + 1);
  Ok(())
}

async fn test_authenticate(port: &u16) -> Result<()> {
  let mut client = InvocationServiceClient::connect(format!("http://127.0.0.1:{}", port)).await?;
  info!("Connected to server");
  let username = "jane2@domain.com";
  let user_id = "someid2";
  let password = "password123";
  let _user_id2 = create_user(&mut client, username, user_id, password).await?;
  let session_in = "session in";
  let session_out = authenticate(&mut client, username, password, session_in).await?;
  trace!("Session is {}", session_out);
  assert_eq!(session_out, session_in);

  Ok(())
}

async fn test_validate_session(port: &u16) -> Result<()> {
  let mut client = InvocationServiceClient::connect(format!("http://127.0.0.1:{}", port)).await?;
  info!("Connected to server");
  let username = "jane3@domain.com";
  let uid0 = "someid3";
  let password = "password123";
  let uid1 = create_user(&mut client, username, uid0, password).await?;
  let session_in = "session in";
  let session = authenticate(&mut client, username, password, session_in).await?;
  trace!("Session is {:?}", session);
  let uid2 = validate_session(&mut client, &session).await?;
  assert_eq!(uid0, uid1);
  assert_eq!(uid1, uid2);

  Ok(())
}

async fn test_update_permissions(port: &u16) -> Result<()> {
  let mut client = InvocationServiceClient::connect(format!("http://127.0.0.1:{}", port)).await?;
  info!("Connected to server");
  let username = "jane4@domain.com";
  let uid0 = "someid4";
  let password = "password123";
  let uid1 = create_user(&mut client, username, uid0, password).await?;
  let perms_in = ["can_do"];
  let perms_out = update_permissions(&mut client, &uid1, &perms_in).await?;
  trace!("perms out {:?}", perms_out);
  assert_eq!(perms_out, perms_in);
  let perms_out = list_permissions(&mut client, &uid1).await?;
  trace!("listed perms {:?}", perms_out);
  assert_eq!(perms_out, perms_in);
  let packet = has_permission(&mut client, &uid1, "can_do").await?;
  trace!("has perm packet {:?}", packet);
  let uid2: String = packet.try_into()?;
  assert_eq!(uid1, uid2);
  let packet = has_permission(&mut client, &uid1, "can't_do").await?;
  trace!("has perm packet {:?}", packet);
  assert!(matches!(packet, Packet::V0(v0::Payload::Error(_))));

  Ok(())
}

pub async fn test_api(provider: impl RpcHandler + 'static) -> Result<()> {
  let socket = vino_rpc::bind_new_socket()?;
  let port = socket.local_addr()?.port();
  vino_rpc::make_rpc_server(socket, provider);

  let components = list_components(&port).await?;
  // println!("Reported components: {:#?}", components);
  // assert_eq!(components.len(), 3);
  println!("Testing create-user");
  test_create_user(&port).await?;
  println!("Testing list and remove");
  test_list_users(&port).await?;
  println!("Testing authenticate");
  test_authenticate(&port).await?;
  println!("Testing validate-sessions");
  test_validate_session(&port).await?;
  println!("Testing update-permissions");
  test_update_permissions(&port).await?;
  Ok(())
}
