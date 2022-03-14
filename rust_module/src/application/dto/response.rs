use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::domain::entity::response::{DataResponse, MediaResponse, PeerResponse};
use crate::domain::entity::{
    AnswerResult, DataConnectionEventEnum, DataConnectionId, DataConnectionIdWrapper,
    DataConnectionStatus, DataId, DataIdWrapper, MediaConnectionEventEnum,
    MediaConnectionIdWrapper, MediaConnectionStatus, MediaId, MediaIdWrapper, PeerEventEnum,
    PeerInfo, PeerStatusMessage, RtcpId, RtcpIdWrapper, SocketInfo,
};
use crate::error;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "command")]
pub(crate) enum PeerResponseDto {
    #[serde(rename = "CREATE")]
    Create(PeerInfo),
    #[serde(rename = "STATUS")]
    Status(PeerStatusMessage),
    #[serde(rename = "DELETE")]
    Delete(PeerInfo),
    #[serde(rename = "EVENT")]
    Event(PeerEventEnum),
}

impl PeerResponseDto {
    pub(crate) fn from_entity(entity: PeerResponse) -> Self {
        match entity {
            PeerResponse::Create(item) => PeerResponseDto::Create(item),
            PeerResponse::Delete(item) => PeerResponseDto::Delete(item),
            PeerResponse::Status(item) => PeerResponseDto::Status(item),
            PeerResponse::Event(item) => PeerResponseDto::Event(item),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "command")]
pub(crate) enum MediaResponseDto {
    #[serde(rename = "CONTENT_CREATE")]
    ContentCreate(SocketInfo<MediaId>),
    #[serde(rename = "CONTENT_DELETE")]
    ContentDelete(MediaIdWrapper),
    #[serde(rename = "RTCP_CREATE")]
    RtcpCreate(SocketInfo<RtcpId>),
    #[serde(rename = "RTCP_DELETE")]
    RtcpDelete(RtcpIdWrapper),
    #[serde(rename = "CALL")]
    Call(MediaConnectionIdWrapper),
    #[serde(rename = "ANSWER")]
    Answer(AnswerResult),
    #[serde(rename = "EVENT")]
    Event(MediaConnectionEventEnum),
    #[serde(rename = "STATUS")]
    Status(MediaConnectionStatus),
}

impl MediaResponseDto {
    pub(crate) fn from_entity(entity: MediaResponse) -> Self {
        match entity {
            MediaResponse::ContentCreate(item) => MediaResponseDto::ContentCreate(item),
            MediaResponse::ContentDelete(item) => MediaResponseDto::ContentDelete(item),
            MediaResponse::RtcpCreate(item) => MediaResponseDto::RtcpCreate(item),
            MediaResponse::RtcpDelete(item) => MediaResponseDto::RtcpDelete(item),
            MediaResponse::Call(item) => MediaResponseDto::Call(item),
            MediaResponse::Answer(item) => MediaResponseDto::Answer(item),
            MediaResponse::Event(item) => MediaResponseDto::Event(item),
            MediaResponse::Status(item) => MediaResponseDto::Status(item),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct DataConnectionResponse {
    pub(crate) data_connection_id: DataConnectionId,
    pub(crate) source_topic_name: String,
    pub(crate) source_ip: String,
    pub(crate) source_port: u16,
    pub(crate) destination_topic_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "command")]
pub(crate) enum DataResponseDto {
    #[serde(rename = "CREATE")]
    Create(SocketInfo<DataId>),
    #[serde(rename = "CONNECT")]
    Connect(DataConnectionIdWrapper),
    #[serde(rename = "DELETE")]
    Delete(DataIdWrapper),
    #[serde(rename = "DISCONNECT")]
    Disconnect(DataConnectionIdWrapper),
    #[serde(rename = "REDIRECT")]
    Redirect(DataConnectionIdWrapper),
    #[serde(rename = "EVENT")]
    Event(DataConnectionEventEnum),
    #[serde(rename = "STATUS")]
    Status(DataConnectionStatus),
}

impl DataResponseDto {
    pub(crate) fn from_entity(entity: DataResponse) -> Self {
        match entity {
            DataResponse::Create(item) => DataResponseDto::Create(item),
            DataResponse::Connect(item) => DataResponseDto::Connect(item),
            DataResponse::Delete(item) => DataResponseDto::Delete(item),
            DataResponse::Disconnect(item) => DataResponseDto::Disconnect(item),
            DataResponse::Redirect(item) => DataResponseDto::Redirect(item),
            DataResponse::Event(item) => DataResponseDto::Event(item),
            DataResponse::Status(item) => DataResponseDto::Status(item),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub(crate) enum ResponseDto {
    #[serde(rename = "PEER")]
    Peer(PeerResponseDto),
    #[serde(rename = "MEDIA")]
    Media(MediaResponseDto),
    #[serde(rename = "DATA")]
    Data(DataResponseDto),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum ResponseDtoResult {
    Success(ResponseDto),
    Error(String),
}

impl ResponseDtoResult {
    pub(crate) fn from_str(json: &str) -> Result<ResponseDtoResult, error::Error> {
        #[allow(dead_code)]
        #[derive(Deserialize)]
        struct ResponseMessageStruct {
            is_success: bool,
            result: serde_json::Value,
        }
        let value = serde_json::from_str::<ResponseMessageStruct>(json)
            .map_err(|e| error::Error::SerdeError { error: e })?;
        match value.is_success {
            true => {
                let content: ResponseDto = serde_json::from_value(value.result)
                    .map_err(|e| error::Error::SerdeError { error: e })?;
                Ok(ResponseDtoResult::Success(content))
            }
            _ => {
                let content: String = serde_json::from_value(value.result)
                    .map_err(|e| error::Error::SerdeError { error: e })?;
                Ok(ResponseDtoResult::Error(content))
            }
        }
    }

    pub(crate) fn to_string(&self) -> Result<String, error::Error> {
        serde_json::to_string(self).map_err(|e| error::Error::SerdeError { error: e })
    }
}

impl Serialize for ResponseDtoResult {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Person", 2)?;
        match self {
            ResponseDtoResult::Success(value) => {
                state.serialize_field("is_success", &true)?;
                state.serialize_field("result", &value)?;
            }
            ResponseDtoResult::Error(value) => {
                state.serialize_field("is_success", &false)?;
                state.serialize_field("result", &value)?;
            }
        }
        state.end()
    }
}