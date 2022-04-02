use std::ffi::CString;
use std::sync::Arc;

use async_trait::async_trait;
use shaku::Component;
use skyway_webrtc_gateway_caller::prelude::data::RedirectParams;
use skyway_webrtc_gateway_caller::prelude::response_parser::DataResponse;

use crate::application::dto::request::{DataRequestDto, RequestDto};
use crate::application::dto::response::{DataResponseDto, ResponseDto, ResponseDtoResult};
use crate::application::factory::Factory;
use crate::application::usecase::Service;
use crate::domain::entity::request::{DataRequest, Request};
use crate::domain::entity::response::{Response, ResponseResult};
use crate::domain::entity::{
    DataIdWrapper, PhantomId, SerializableId, SerializableSocket, SocketInfo,
};
use crate::domain::repository::Repository;
use crate::ffi::global_params::DataConnectionResponse;
use crate::ffi::{DestinationParameters, SourceParameters, TopicParameters};
use crate::utils::{available_port, CallbackCaller};
use crate::{error, GlobalState};

#[derive(Component)]
#[shaku(interface = Service)]
pub(crate) struct Redirect {
    #[shaku(inject)]
    repository: Arc<dyn Repository>,
    #[shaku(inject)]
    state: Arc<dyn GlobalState>,
    #[shaku(inject)]
    factory: Arc<dyn Factory>,
    #[shaku(inject)]
    callback: Arc<dyn CallbackCaller>,
}

#[async_trait]
impl Service for Redirect {
    async fn execute(&self, request: RequestDto) -> Result<ResponseDtoResult, error::Error> {
        if let RequestDto::Data(DataRequestDto::Redirect {
            params: redirect_params,
        }) = request
        {
            // 確立されたDataChannelに対し、以下の内容を実施する
            // 1. Dataポートを開放させ、DataChannelへのSourceとして利用する
            //    Source TopicのTopic名にはDataIdを利用する
            // 2. GWから受信したデータをEnd-User-Programにデータを渡すためのDest Topicのパラメータを生成する
            // 3. Redirectの実行
            // 4. Source, Destination Topicの保管

            // 1.は単独で実施可能なので最初に行う
            let (data_id, address, port) = {
                let create_data_param = RequestDto::Data(DataRequestDto::Create);
                let service = self.factory.create_service(&create_data_param);
                let result = service.execute(create_data_param).await?;
                if let ResponseDtoResult::Success(ResponseDto::Data(DataResponseDto::Create(
                    socket,
                ))) = result
                {
                    (
                        socket.get_id().expect("failed to open data port"),
                        socket.ip(),
                        socket.port(),
                    )
                } else {
                    let message = format!("create data failed {:?}", result);
                    return Err(error::Error::create_local_error(&message));
                }
            };

            // Source TopicのIDを生成
            // topic名には-が使えないので_に置換する
            let source_topic_name = data_id.as_str().replace("-", "_");

            // 2. GWから受信したデータをEnd-User-Programにデータを渡すためのDest Topicのパラメータを生成する
            //    GWからこのポートに転送されたデータが最終的にエンドユーザに届けられる
            let available_port = available_port().expect("bind port failed");

            // 3. Redirectの実行
            // REDIRECT APIを呼ぶためのパラメータ生成
            // Dest ObjectのUDPソケット情報が必要なので、このタイミングで実施する
            let params = {
                let params = RedirectParams {
                    data_connection_id: redirect_params.data_connection_id,
                    feed_params: Some(DataIdWrapper { data_id }),
                    redirect_params: Some(
                        SocketInfo::<PhantomId>::try_create(None, "127.0.0.1", available_port)
                            .unwrap(),
                    ),
                };
                Request::Data(DataRequest::Redirect { params })
            };
            let result = self.repository.register(params).await?;

            // 4. Redirectが成功した場合は、C++側に通知してTopicを開かせた後、
            //    Eventの処理やDisconnect時に利用するため、Topicに関する情報を登録する
            match result {
                // Redirectに成功した場合
                ResponseResult::Success(Response::Data(DataResponse::Redirect(params))) => {
                    // Source Topicの情報
                    let source_parameters = SourceParameters {
                        source_topic_name: CString::new(source_topic_name.as_str())
                            .unwrap()
                            .into_raw(),
                        destination_address: CString::new(address.to_string().as_str())
                            .unwrap()
                            .into_raw(),
                        destination_port: port,
                    };

                    // Destination Topicの情報
                    let destination_parameters = DestinationParameters {
                        source_port: available_port,
                        destination_topic_name: CString::new(
                            redirect_params.destination_topic.as_str(),
                        )
                        .unwrap()
                        .into_raw(),
                    };

                    // 1つに束ねてC++に渡す
                    let topic_parameters = TopicParameters {
                        data_connection_id: CString::new(params.data_connection_id.as_str())
                            .unwrap()
                            .into_raw(),
                        source_parameters,
                        destination_parameters,
                    };
                    self.callback.data_callback(topic_parameters);

                    // Topicの情報を保管
                    let response = DataConnectionResponse {
                        data_connection_id: params.data_connection_id.clone(),
                        source_topic_name,
                        source_ip: address.to_string(),
                        source_port: port,
                        destination_topic_name: redirect_params.destination_topic,
                    };
                    self.state
                        .store_topic(params.data_connection_id.clone(), response);

                    return Ok(ResponseDtoResult::Success(ResponseDto::Data(
                        DataResponseDto::Redirect(params),
                    )));
                }
                ResponseResult::Error(message) => return Ok(ResponseDtoResult::Error(message)),
                _ => {
                    // 別のAPIの成功結果が得られることはない
                    unreachable!()
                }
            }
        }

        return Err(error::Error::create_local_error("invalid parameters"));
    }
}

#[cfg(test)]
mod redirect_data_test {
    use shaku::HasComponent;

    use super::*;
    use crate::application::factory::MockFactory;
    use crate::application::usecase::MockService;
    use crate::di::*;
    use crate::domain::entity::response::{DataResponse, ResponseResult};
    use crate::domain::entity::{DataConnectionId, DataConnectionIdWrapper, DataId, SocketInfo};
    use crate::domain::repository::MockRepository;
    use crate::utils::MockCallbackCaller;
    use crate::MockGlobalState;

    #[tokio::test]
    // eventとして異常な文字列を受信した場合
    async fn success() {
        // 待値の生成
        // DataConnectionResponseを含むRedirectパラメータを受け取れるはずである
        let expected = {
            let value = DataConnectionIdWrapper {
                data_connection_id: DataConnectionId::try_create(
                    "dc-4995f372-fb6a-4196-b30a-ce11e5c7f56c",
                )
                .unwrap(),
            };

            ResponseDtoResult::Success(ResponseDto::Data(DataResponseDto::Redirect(value)))
        };

        let request = {
            let message = r#"{
                "type":"DATA",
                "command":"REDIRECT",
                "params":{
                    "data_connection_id":"dc-8bdef7a1-65c8-46be-a82e-37d51c776309",
                    "destination_topic":"destination_topic"
                }
            }"#;

            RequestDto::from_str(&message).unwrap()
        };

        let mut factory = MockFactory::new();
        factory.expect_create_service().times(1).returning(|_| {
            let mut mock_service = MockService::new();
            mock_service.expect_execute().returning(|_| {
                let socket = SocketInfo::<DataId>::try_create(
                    Some("da-06cf1d26-0ef0-4b03-aca6-933027d434c2".to_string()),
                    "127.0.0.1",
                    10000,
                )
                .unwrap();
                Ok(ResponseDtoResult::Success(ResponseDto::Data(
                    DataResponseDto::Create(socket),
                )))
            });
            Arc::new(mock_service)
        });

        let mut repository = MockRepository::new();
        repository.expect_register().times(1).returning(|_| {
            // redirectのmock
            // 成功し、DataConnectionIdを返すケース
            Ok(ResponseResult::Success(Response::Data(
                DataResponse::Redirect(DataConnectionIdWrapper {
                    data_connection_id: DataConnectionId::try_create(
                        "dc-4995f372-fb6a-4196-b30a-ce11e5c7f56c",
                    )
                    .unwrap(),
                }),
            )))
        });

        let mut caller = MockCallbackCaller::new();
        caller.expect_data_callback().times(1).returning(|_| ());

        let mut state = MockGlobalState::new();
        state.expect_store_topic().times(1).returning(|_, _| ());

        // サービスの生成
        let module = DataRedirectService::builder()
            .with_component_override::<dyn Factory>(Box::new(factory))
            .with_component_override::<dyn Repository>(Box::new(repository))
            .with_component_override::<dyn CallbackCaller>(Box::new(caller))
            .with_component_override::<dyn GlobalState>(Box::new(state))
            .build();
        let service: &dyn Service = module.resolve_ref();

        let result = service.execute(request).await;
        assert_eq!(result.unwrap(), expected);
    }
}
