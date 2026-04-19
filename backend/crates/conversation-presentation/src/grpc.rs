//! gRPC handler for the Conversation context.

use crate::ConversationServices;
use audit::Actor;
use conversation_application::{
    EndConversationError, EndConversationInput, GetHistoryError, GetHistoryInput,
    ListConversationsError, ListConversationsInput, SendMessageError, SendMessageInput,
    StartConversationError, StartConversationInput,
};
use conversation_contracts::v1::{
    ConversationStatus as ProtoStatus, ConversationSummary, EmotionalTone as ProtoTone,
    EndConversationRequest, EndConversationResponse, GetHistoryRequest, GetHistoryResponse,
    ListConversationsRequest, ListConversationsResponse, Message as ProtoMessage,
    Role as ProtoRole, SendMessageRequest, SendMessageResponse, StartConversationRequest,
    StartConversationResponse,
    conversation_service_server::{ConversationService, ConversationServiceServer},
};
use conversation_domain::{
    Conversation, ConversationStatus, EmotionalTone, Message, Role, conversation::UserRef,
};
use kernel::EntityId;
use prost_types::Timestamp;
use std::str::FromStr;
use tonic::{Request, Response, Status};

pub struct ConversationGrpc {
    services: ConversationServices,
}

impl ConversationGrpc {
    #[must_use]
    pub fn new(services: ConversationServices) -> ConversationServiceServer<Self> {
        ConversationServiceServer::new(Self { services })
    }
}

fn ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: i32::try_from(dt.timestamp_subsec_nanos()).unwrap_or(0),
    }
}

fn to_tone(t: EmotionalTone) -> i32 {
    match t {
        EmotionalTone::Neutral => ProtoTone::Neutral as i32,
        EmotionalTone::Happy => ProtoTone::Happy as i32,
        EmotionalTone::Sad => ProtoTone::Sad as i32,
        EmotionalTone::Angry => ProtoTone::Angry as i32,
        EmotionalTone::Anxious => ProtoTone::Anxious as i32,
        EmotionalTone::Surprised => ProtoTone::Surprised as i32,
        EmotionalTone::Calm => ProtoTone::Calm as i32,
        EmotionalTone::Excited => ProtoTone::Excited as i32,
    }
}

fn to_role(r: Role) -> i32 {
    match r {
        Role::User => ProtoRole::User as i32,
        Role::Assistant => ProtoRole::Assistant as i32,
        Role::System => ProtoRole::System as i32,
    }
}

fn to_status(s: ConversationStatus) -> i32 {
    match s {
        ConversationStatus::Active => ProtoStatus::Active as i32,
        ConversationStatus::Ended => ProtoStatus::Ended as i32,
    }
}

fn to_proto_message(m: &Message) -> ProtoMessage {
    ProtoMessage {
        id: m.id().to_string(),
        role: to_role(m.role()),
        body: m.body().to_string(),
        tone: to_tone(m.tone()),
        sent_at: Some(ts(m.sent_at())),
    }
}

fn to_proto_summary(c: &Conversation) -> ConversationSummary {
    ConversationSummary {
        id: c.id().to_string(),
        user_id: c.user_id().to_string(),
        title: c.title().unwrap_or("").to_string(),
        status: to_status(c.status()),
        message_count: c.message_count(),
        started_at: Some(ts(c.started_at())),
        ended_at: c.ended_at().map(ts),
    }
}

fn anon_actor() -> EntityId<Actor> {
    EntityId::<Actor>::from_uuid(uuid::Uuid::nil())
}

#[tonic::async_trait]
impl ConversationService for ConversationGrpc {
    async fn start_conversation(
        &self,
        request: Request<StartConversationRequest>,
    ) -> Result<Response<StartConversationResponse>, Status> {
        let req = request.into_inner();
        let user_id = EntityId::<UserRef>::from_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("malformed user_id"))?;
        let title = if req.title.is_empty() {
            None
        } else {
            Some(req.title)
        };
        let out = self
            .services
            .start
            .execute(StartConversationInput {
                user_id,
                title,
                actor_id: anon_actor(),
            })
            .await
            .map_err(|e| match e {
                StartConversationError::Repository(_) => Status::unavailable("storage"),
                StartConversationError::Audit(_) => Status::internal("audit"),
            })?;
        Ok(Response::new(StartConversationResponse {
            conversation_id: out.conversation_id.to_string(),
        }))
    }

    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let req = request.into_inner();
        let conversation_id = EntityId::<Conversation>::from_str(&req.conversation_id)
            .map_err(|_| Status::invalid_argument("malformed conversation_id"))?;
        let out = self
            .services
            .send
            .execute(SendMessageInput {
                conversation_id,
                body: req.body,
                actor_id: anon_actor(),
            })
            .await
            .map_err(|e| match e {
                SendMessageError::NotFound => Status::not_found("conversation not found"),
                SendMessageError::Domain(d) => Status::invalid_argument(d.to_string()),
                SendMessageError::Repository(_) => Status::unavailable("storage"),
                SendMessageError::Llm(_) => Status::unavailable("llm"),
                SendMessageError::Audit(_) => Status::internal("audit"),
            })?;
        Ok(Response::new(SendMessageResponse {
            reply: out.reply,
            tone: to_tone(out.tone),
        }))
    }

    async fn end_conversation(
        &self,
        request: Request<EndConversationRequest>,
    ) -> Result<Response<EndConversationResponse>, Status> {
        let req = request.into_inner();
        let conversation_id = EntityId::<Conversation>::from_str(&req.conversation_id)
            .map_err(|_| Status::invalid_argument("malformed conversation_id"))?;
        self.services
            .end
            .execute(EndConversationInput {
                conversation_id,
                actor_id: anon_actor(),
            })
            .await
            .map_err(|e| match e {
                EndConversationError::NotFound => Status::not_found("conversation not found"),
                EndConversationError::Repository(_) => Status::unavailable("storage"),
                EndConversationError::Audit(_) => Status::internal("audit"),
            })?;
        Ok(Response::new(EndConversationResponse {}))
    }

    async fn get_history(
        &self,
        request: Request<GetHistoryRequest>,
    ) -> Result<Response<GetHistoryResponse>, Status> {
        let req = request.into_inner();
        let conversation_id = EntityId::<Conversation>::from_str(&req.conversation_id)
            .map_err(|_| Status::invalid_argument("malformed conversation_id"))?;
        let out = self
            .services
            .history
            .execute(GetHistoryInput {
                conversation_id,
                limit: if req.limit == 0 { 50 } else { req.limit },
            })
            .await
            .map_err(|e| match e {
                GetHistoryError::Repository(_) => Status::unavailable("storage"),
            })?;
        Ok(Response::new(GetHistoryResponse {
            messages: out.messages.iter().map(to_proto_message).collect(),
        }))
    }

    async fn list_conversations(
        &self,
        request: Request<ListConversationsRequest>,
    ) -> Result<Response<ListConversationsResponse>, Status> {
        let req = request.into_inner();
        let user_id = EntityId::<UserRef>::from_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("malformed user_id"))?;
        let out = self
            .services
            .list
            .execute(ListConversationsInput {
                user_id,
                limit: if req.limit == 0 { 20 } else { req.limit },
            })
            .await
            .map_err(|e| match e {
                ListConversationsError::Repository(_) => Status::unavailable("storage"),
            })?;
        Ok(Response::new(ListConversationsResponse {
            conversations: out.conversations.iter().map(to_proto_summary).collect(),
        }))
    }
}
