use crate::protocol::chat;

use flatbuffers::{FlatBufferBuilder, WIPOffset};

pub fn env_with_list_response<'a>(
    fbb: &mut FlatBufferBuilder<'a>,
    summaries: &[(String, usize, usize)],
) -> WIPOffset<chat::Envelope<'a>> {
    let conv_offsets: Vec<_> = summaries
        .iter()
        .map(|(id, user_count, message_count)| {
            let s = fbb.create_string(id);
            chat::ConversationSummary::create(
                fbb,
                &chat::ConversationSummaryArgs {
                    id: Some(s),
                    user_count: *user_count as u32,
                    message_count: *message_count as u32,
                },
            )
        })
        .collect();
    let conv_vec = fbb.create_vector(&conv_offsets);

    let lr = chat::ListResponse::create(
        fbb,
        &chat::ListResponseArgs {
            conversations: Some(conv_vec),
        },
    );

    chat::Envelope::create(
        fbb,
        &chat::EnvelopeArgs {
            frame_type: chat::Message::ListResponse,
            frame: Some(lr.as_union_value()),
        },
    )
}

pub fn env_with_chat<'a>(
    fbb: &mut FlatBufferBuilder<'a>,
    text: &str,
) -> WIPOffset<chat::Envelope<'a>> {
    let s = fbb.create_string(text);
    let cm = chat::ChatMessage::create(fbb, &chat::ChatMessageArgs { text: Some(s) });

    chat::Envelope::create(
        fbb,
        &chat::EnvelopeArgs {
            frame_type: chat::Message::ChatMessage,
            frame: Some(cm.as_union_value()),
        },
    )
}

pub fn env_with_error<'a>(
    fbb: &mut FlatBufferBuilder<'a>,
    msg: &str,
) -> WIPOffset<chat::Envelope<'a>> {
    let s = fbb.create_string(msg);
    let er = chat::Error::create(fbb, &chat::ErrorArgs { message: Some(s) });

    chat::Envelope::create(
        fbb,
        &chat::EnvelopeArgs {
            frame_type: chat::Message::Error,
            frame: Some(er.as_union_value()),
        },
    )
}
