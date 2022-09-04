use std::thread::sleep;
use std::time::Duration;

use shaku::HasComponent;

use super::EventReceiveImpl;
use crate::application::dto::request::RequestDto;
use crate::application::dto::response::{
    DataResponseDto, PeerConnectionEventDto, PeerEventEnumDto, PeerResponseDto, ResponseDto,
    ResponseDtoResult,
};
use crate::di::*;
use crate::domain::entity::response::PeerResponse;
use crate::domain::entity::PeerEventEnum;
use crate::error;
use crate::ffi::rust_to_c_bridge::state_objects::ProgramState;

impl EventReceiveImpl {
    pub(crate) async fn process_peer_event(
        &self,
        response: PeerResponse,
    ) -> Result<PeerResponseDto, error::Error> {
        match response {
            PeerResponse::Event(PeerEventEnum::OPEN(event)) => {
                Ok(PeerResponseDto::Event(PeerEventEnumDto::OPEN(event)))
            }
            PeerResponse::Event(PeerEventEnum::CLOSE(close)) => {
                std::thread::spawn(|| {
                    sleep(Duration::from_millis(100));
                    let module = CppObjctsModule::builder().build();
                    let state: &dyn ProgramState = module.resolve_ref();
                    state.shutdown();
                });

                Ok(PeerResponseDto::Event(PeerEventEnumDto::CLOSE(close)))
            }
            PeerResponse::Event(PeerEventEnum::CONNECTION(connection)) => {
                use serde::{Deserialize, Serialize};

                use crate::application::dto::request::DataRequestDto;
                use crate::application::Factory;

                let module = GeneralFactory::builder().build();
                let factory: &dyn Factory = module.resolve_ref();

                let request_dto = RequestDto::Data(DataRequestDto::Status {
                    params: connection.data_params.clone(),
                });
                let service = factory.create_service(&request_dto);
                let result = service.execute(request_dto).await;
                if let Ok(ResponseDtoResult::Success(ResponseDto::Data(DataResponseDto::Status(
                    status,
                )))) = result
                {
                    let message = serde_json::to_string(&status).unwrap();
                    let event_dto = PeerConnectionEventDto {
                        params: connection.params,
                        data_params: connection.data_params,
                        status,
                    };
                    Ok(PeerResponseDto::Event(PeerEventEnumDto::CONNECTION(
                        event_dto,
                    )))
                } else {
                    let message = format!("connection request is received from {}. But failed to get DataConnection Status.", connection.params.peer_id().as_str());
                    Err(error::Error::create_local_error(&message))
                }
            }
            PeerResponse::Event(PeerEventEnum::CALL(event)) => {
                Ok(PeerResponseDto::Event(PeerEventEnumDto::CALL(event)))
            }
            PeerResponse::Event(PeerEventEnum::TIMEOUT) => unreachable!(),
            _ => {
                let message = format!("Non-Event object is processed in EventReceiveImpl as Peer");
                self.logger.error(&message);
                unreachable!()
            }
        }
    }
}
