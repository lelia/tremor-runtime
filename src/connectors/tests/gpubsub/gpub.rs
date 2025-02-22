// Copyright 2022, The Tremor Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::connectors::impls::gpubsub::producer::Builder;
use crate::connectors::tests::ConnectorHarness;
use crate::errors::Result;
use crate::instance::State;
use async_std::prelude::FutureExt;
use googapis::google::pubsub::v1::publisher_client::PublisherClient;
use googapis::google::pubsub::v1::subscriber_client::SubscriberClient;
use googapis::google::pubsub::v1::{PullRequest, Subscription, Topic};
use serial_test::serial;
use std::collections::HashSet;
use std::time::Duration;
use testcontainers::clients::Cli;
use testcontainers::RunnableImage;
use tonic::transport::Channel;
use tremor_common::ports::IN;
use tremor_pipeline::{Event, EventId};
use tremor_value::{literal, Value};

#[async_std::test]
#[serial(gpubsub)]
async fn no_connection() -> Result<()> {
    serial_test::set_max_wait(Duration::from_secs(600));
    let _ = env_logger::try_init();
    let connector_yaml = literal!({
        "codec": "binary",
        "config":{
            "url": "https://localhost:9090",
            "connect_timeout": 100000000,
            "topic": "projects/xxx/topics/test-a",
            "skip_authentication": false
        }
    });

    let harness =
        ConnectorHarness::new(function_name!(), &Builder::default(), &connector_yaml).await?;
    assert!(harness.start().await.is_err());
    Ok(())
}

#[async_std::test]
#[serial(gpubsub)]
async fn no_hostname() -> Result<()> {
    serial_test::set_max_wait(Duration::from_secs(600));
    let _ = env_logger::try_init();
    let connector_yaml = literal!({
        "codec": "binary",
        "config":{
            "url": "file:///etc/passwd",
            "connect_timeout": 100000000,
            "topic": "projects/xxx/topics/test-a",
            "skip_authentication": false
        }
    });

    let harness =
        ConnectorHarness::new(function_name!(), &Builder::default(), &connector_yaml).await;

    assert!(harness.is_err());

    Ok(())
}

#[async_std::test]
#[serial(gpubsub)]
async fn simple_publish() -> Result<()> {
    serial_test::set_max_wait(Duration::from_secs(600));

    let _ = env_logger::try_init();

    let runner = Cli::docker();

    let (pubsub, pubsub_args) =
        testcontainers::images::google_cloud_sdk_emulators::CloudSdk::pubsub();
    let runnable_image = RunnableImage::from((pubsub, pubsub_args));
    let container = runner.run(runnable_image);

    let port = container
        .get_host_port_ipv4(testcontainers::images::google_cloud_sdk_emulators::PUBSUB_PORT);
    let endpoint = format!("http://localhost:{}", port);
    let endpoint_clone = endpoint.clone();

    let connector_yaml: Value = literal!({
        "codec": "binary",
        "config":{
            "url": endpoint,
            "connect_timeout": 30000000000u64,
            "topic": "projects/test/topics/test",
            "skip_authentication": true
        }
    });

    let channel = Channel::from_shared(endpoint_clone)?.connect().await?;
    let mut subscriber = SubscriberClient::new(channel.clone());
    let mut publisher = PublisherClient::new(channel.clone());
    publisher
        .create_topic(Topic {
            name: "projects/test/topics/test".to_string(),
            labels: Default::default(),
            message_storage_policy: None,
            kms_key_name: "".to_string(),
            schema_settings: None,
            satisfies_pzs: false,
            message_retention_duration: None,
        })
        .await?;
    subscriber
        .create_subscription(Subscription {
            name: "projects/test/subscriptions/test-subscription-a".to_string(),
            topic: "projects/test/topics/test".to_string(),
            push_config: None,
            ack_deadline_seconds: 0,
            retain_acked_messages: false,
            message_retention_duration: None,
            labels: Default::default(),
            enable_message_ordering: false,
            expiration_policy: None,
            filter: "".to_string(),
            dead_letter_policy: None,
            retry_policy: None,
            detached: false,
            topic_message_retention_duration: None,
        })
        .await?;

    let harness =
        ConnectorHarness::new(function_name!(), &Builder::default(), &connector_yaml).await?;
    harness.start().await?;
    harness.wait_for_connected().await?;
    harness.consume_initial_sink_contraflow().await?;

    for i in 0..100 {
        let event = Event {
            id: EventId::default(),
            data: (Value::String(format!("Event {}", i).into()), literal!({})).into(),
            ..Event::default()
        };
        harness.send_to_sink(event, IN).await?;
    }

    let mut received_messages = HashSet::new();
    let mut iter_count = 0;

    while received_messages.len() < 100 && iter_count <= 100 {
        let result = subscriber
            .pull(PullRequest {
                subscription: "projects/test/subscriptions/test-subscription-a".to_string(),
                max_messages: 1000,
                ..Default::default()
            })
            .await?;

        for msg in result.into_inner().received_messages {
            let body = msg.message.unwrap();
            received_messages.insert(String::from_utf8(body.data).unwrap());
        }

        iter_count += 1;
    }

    if received_messages.len() != 100 {
        panic!(
            "Received an unexpected number of messages - {}",
            received_messages.len()
        );
    }

    harness.stop().await?;

    Ok(())
}

#[async_std::test]
#[serial(gpubsub)]
async fn simple_publish_with_timeout() -> Result<()> {
    serial_test::set_max_wait(Duration::from_secs(600));

    let _ = env_logger::try_init();

    let runner = Cli::docker();

    let (pubsub, pubsub_args) =
        testcontainers::images::google_cloud_sdk_emulators::CloudSdk::pubsub();
    let runnable_image = RunnableImage::from((pubsub, pubsub_args));
    let container = runner.run(runnable_image);

    let port = container
        .get_host_port_ipv4(testcontainers::images::google_cloud_sdk_emulators::PUBSUB_PORT);
    let endpoint = format!("http://localhost:{}", port);
    let endpoint_clone = endpoint.clone();

    let connector_yaml: Value = literal!({
        "codec": "binary",
        "config":{
            "url": endpoint,
            "connect_timeout": 30000000000u64,
            "topic": "projects/test/topics/test",
            "skip_authentication": true
        }
    });

    let channel = Channel::from_shared(endpoint_clone)?.connect().await?;
    let mut subscriber = SubscriberClient::new(channel.clone());
    let mut publisher = PublisherClient::new(channel.clone());
    publisher
        .create_topic(Topic {
            name: "projects/test/topics/test".to_string(),
            labels: Default::default(),
            message_storage_policy: None,
            kms_key_name: "".to_string(),
            schema_settings: None,
            satisfies_pzs: false,
            message_retention_duration: None,
        })
        .await?;
    subscriber
        .create_subscription(Subscription {
            name: "projects/test/subscriptions/test-subscription-a".to_string(),
            topic: "projects/test/topics/test".to_string(),
            push_config: None,
            ack_deadline_seconds: 0,
            retain_acked_messages: false,
            message_retention_duration: None,
            labels: Default::default(),
            enable_message_ordering: false,
            expiration_policy: None,
            filter: "".to_string(),
            dead_letter_policy: None,
            retry_policy: None,
            detached: false,
            topic_message_retention_duration: None,
        })
        .await?;

    let harness =
        ConnectorHarness::new(function_name!(), &Builder::default(), &connector_yaml).await?;
    harness.start().await?;
    harness.wait_for_connected().await?;
    harness.consume_initial_sink_contraflow().await?;

    drop(container);

    let event = Event {
        id: EventId::default(),
        data: (Value::String(format!("Event X").into()), literal!({})).into(),
        ..Event::default()
    };
    harness.send_to_sink(event, IN).await?;
    harness
        .wait_for_state(State::Failed)
        .timeout(Duration::from_secs(10))
        .await?
        .unwrap();

    harness.stop().await?;

    Ok(())
}
