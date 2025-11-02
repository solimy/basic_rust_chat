use crate::protocol::chat;

use flatbuffers::{FlatBufferBuilder, WIPOffset};

pub fn env_with_list_request<'a>(fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<chat::Envelope<'a>> {
    let lr = chat::ListRequest::create(fbb, &chat::ListRequestArgs {});
    chat::Envelope::create(
        fbb,
        &chat::EnvelopeArgs {
            frame_type: chat::Message::ListRequest,
            frame: Some(lr.as_union_value()),
        },
    )
}

pub fn env_with_join_request<'a>(
    fbb: &mut FlatBufferBuilder<'a>,
    conversation_id: &str,
) -> WIPOffset<chat::Envelope<'a>> {
    let cid = fbb.create_string(conversation_id);
    let jr = chat::JoinRequest::create(
        fbb,
        &chat::JoinRequestArgs {
            conversation_id: Some(cid),
        },
    );
    chat::Envelope::create(
        fbb,
        &chat::EnvelopeArgs {
            frame_type: chat::Message::JoinRequest,
            frame: Some(jr.as_union_value()),
        },
    )
}

pub fn env_with_client_text<'a>(
    fbb: &mut FlatBufferBuilder<'a>,
    text: &str,
) -> WIPOffset<chat::Envelope<'a>> {
    let t = fbb.create_string(text);
    let ct = chat::ClientText::create(fbb, &chat::ClientTextArgs { text: Some(t) });
    chat::Envelope::create(
        fbb,
        &chat::EnvelopeArgs {
            frame_type: chat::Message::ClientText,
            frame: Some(ct.as_union_value()),
        },
    )
}
