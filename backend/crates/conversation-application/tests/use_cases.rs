//! Use-case tests driven by in-memory adapters (§3.2). Covers the happy path
//! plus the two domain invariants the use cases enforce: ended conversations
//! reject new messages, and list/history reads project correctly.

use std::sync::Arc;

use audit::{Actor, AuditError, AuditEvent, AuditPort};
use async_trait::async_trait;
use conversation_application::{
    EndConversation, EndConversationInput, GetHistory, GetHistoryInput, ListConversations,
    ListConversationsInput, SendMessage, SendMessageError, SendMessageInput, StartConversation,
    StartConversationInput,
};
use conversation_domain::{EmotionalTone, conversation::UserRef};
use conversation_infrastructure::in_memory::{
    EchoLlm, InMemoryConversationRepository, InMemoryMessageStore,
};
use kernel::{EntityId, clock::SystemClock};
use std::sync::Mutex;

#[derive(Default)]
struct CapturingAudit(Mutex<Vec<AuditEvent>>);

#[async_trait]
impl AuditPort for CapturingAudit {
    async fn append(&self, event: AuditEvent) -> Result<(), AuditError> {
        self.0.lock().expect("lock").push(event);
        Ok(())
    }
}

struct Fixture {
    start: StartConversation,
    send: SendMessage,
    end: EndConversation,
    history: GetHistory,
    list: ListConversations,
    audit: Arc<CapturingAudit>,
}

fn build() -> Fixture {
    let repo = Arc::new(InMemoryConversationRepository::default());
    let store = Arc::new(InMemoryMessageStore::default());
    let llm = Arc::new(EchoLlm {
        tone: EmotionalTone::Calm,
    });
    let audit = Arc::new(CapturingAudit::default());
    let clock = Arc::new(SystemClock);
    Fixture {
        start: StartConversation::new(repo.clone(), audit.clone(), clock.clone()),
        send: SendMessage::new(
            repo.clone(),
            store.clone(),
            llm,
            audit.clone(),
            clock.clone(),
        ),
        end: EndConversation::new(repo.clone(), audit.clone(), clock.clone()),
        history: GetHistory::new(store),
        list: ListConversations::new(repo),
        audit,
    }
}

#[tokio::test]
async fn start_send_history_end_happy_path() {
    let fx = build();
    let user_id = EntityId::<UserRef>::new();

    let started = fx
        .start
        .execute(StartConversationInput {
            user_id,
            title: Some("hello".into()),
            actor_id: EntityId::<Actor>::new(),
        })
        .await
        .expect("start");

    let sent = fx
        .send
        .execute(SendMessageInput {
            conversation_id: started.conversation_id,
            body: "how are you?".into(),
            actor_id: EntityId::<Actor>::new(),
        })
        .await
        .expect("send");
    assert_eq!(sent.tone, EmotionalTone::Calm);
    assert!(sent.reply.contains("echo:"));

    let hist = fx
        .history
        .execute(GetHistoryInput {
            conversation_id: started.conversation_id,
            limit: 10,
        })
        .await
        .expect("history");
    assert_eq!(hist.messages.len(), 2);

    fx.end
        .execute(EndConversationInput {
            conversation_id: started.conversation_id,
            actor_id: EntityId::<Actor>::new(),
        })
        .await
        .expect("end");

    // start + send + end = 3 audit events (SendMessage emits exactly one).
    let events = fx.audit.0.lock().expect("lock");
    assert_eq!(events.len(), 3);
    let actions: Vec<_> = events.iter().map(|e| e.action.clone()).collect();
    assert!(actions.contains(&"conversation.started".to_string()));
    assert!(actions.contains(&"conversation.message.sent".to_string()));
    assert!(actions.contains(&"conversation.ended".to_string()));
}

#[tokio::test]
async fn send_to_ended_conversation_fails() {
    let fx = build();
    let user_id = EntityId::<UserRef>::new();
    let started = fx
        .start
        .execute(StartConversationInput {
            user_id,
            title: None,
            actor_id: EntityId::<Actor>::new(),
        })
        .await
        .expect("start");
    fx.end
        .execute(EndConversationInput {
            conversation_id: started.conversation_id,
            actor_id: EntityId::<Actor>::new(),
        })
        .await
        .expect("end");

    let result = fx
        .send
        .execute(SendMessageInput {
            conversation_id: started.conversation_id,
            body: "still there?".into(),
            actor_id: EntityId::<Actor>::new(),
        })
        .await;
    assert!(
        matches!(result, Err(SendMessageError::Domain(_))),
        "ended conversation must reject with Domain error"
    );
}

#[tokio::test]
async fn send_with_unknown_conversation_returns_not_found() {
    let fx = build();
    let result = fx
        .send
        .execute(SendMessageInput {
            conversation_id: EntityId::new(),
            body: "hello".into(),
            actor_id: EntityId::<Actor>::new(),
        })
        .await;
    assert!(
        matches!(result, Err(SendMessageError::NotFound)),
        "missing conversation must fail with NotFound"
    );
}

#[tokio::test]
async fn list_conversations_scopes_to_user() {
    let fx = build();
    let mine = EntityId::<UserRef>::new();
    let theirs = EntityId::<UserRef>::new();
    for (owner, title) in [(mine, "a"), (mine, "b"), (theirs, "c")] {
        fx.start
            .execute(StartConversationInput {
                user_id: owner,
                title: Some(title.into()),
                actor_id: EntityId::<Actor>::new(),
            })
            .await
            .expect("start");
    }
    let mine_list = fx
        .list
        .execute(ListConversationsInput {
            user_id: mine,
            limit: 10,
        })
        .await
        .expect("list");
    assert_eq!(mine_list.conversations.len(), 2);

    let theirs_list = fx
        .list
        .execute(ListConversationsInput {
            user_id: theirs,
            limit: 10,
        })
        .await
        .expect("list");
    assert_eq!(theirs_list.conversations.len(), 1);
}
